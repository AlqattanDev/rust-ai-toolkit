use crate::error::Result;
use super::AiClient;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{debug, info};
use tokio::sync::RwLock;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::ai::RequestOptions;
use rand;
use crate::config;
use lazy_static::lazy_static;

/// The maximum time a response should be kept in cache
const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

/// The maximum number of items to keep in the cache
const MAX_CACHE_SIZE: usize = 1000;

// Initialize the global response cache
lazy_static! {
    /// Global response cache for AI completions
    pub static ref RESPONSE_CACHE: RwLock<ResponseCache> = {
        let config = config::get_config().unwrap_or_default();
        let max_size = MAX_CACHE_SIZE;
        let max_memory_mb = config.max_cache_size_mb as usize;
        let max_memory_bytes = max_memory_mb * 1024 * 1024; // Convert MB to bytes
        RwLock::new(ResponseCache::new())
    };
}

/// A cached AI response
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// The cached response text
    pub response: String,
    /// When this response was cached
    pub cached_at: Instant,
}

impl CachedResponse {
    /// Create a new cached response
    pub fn new(response: String) -> Self {
        Self {
            response,
            cached_at: Instant::now(),
        }
    }
    
    /// Check if the cache is still valid
    pub fn is_valid(&self) -> bool {
        self.cached_at.elapsed() < CACHE_TTL
    }
}

/// A simple hash function for prompts
fn hash_prompt(prompt: &str, max_tokens: Option<u32>) -> u64 {
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    if let Some(tokens) = max_tokens {
        tokens.hash(&mut hasher);
    }
    hasher.finish()
}

/// Struct for caching AI responses
#[derive(Default)]
pub struct ResponseCache {
    /// Map of prompt hashes to their cached responses
    cache: HashMap<u64, CachedResponse>,
    /// Queue of keys in order of insertion for LRU eviction
    keys_queue: VecDeque<u64>,
    /// Maximum cache size (number of items)
    max_size: usize,
    /// Total memory usage estimation (rough approximation)
    estimated_memory_usage: usize,
    /// Maximum memory usage in bytes
    max_memory_usage: usize,
}

impl ResponseCache {
    /// Create a new empty response cache
    pub fn new() -> Self {
        // Get the config for cache settings
        let config = config::get_config().unwrap_or_default();
        let max_memory_mb = config.max_cache_size_mb as usize;
        
        Self {
            cache: HashMap::new(),
            keys_queue: VecDeque::with_capacity(MAX_CACHE_SIZE),
            max_size: MAX_CACHE_SIZE,
            estimated_memory_usage: 0,
            max_memory_usage: max_memory_mb * 1024 * 1024, // Convert MB to bytes
        }
    }
    
    /// Get a cached response if it exists and is valid
    pub fn get(&self, prompt: &str, max_tokens: Option<u32>) -> Option<String> {
        let key = hash_prompt(prompt, max_tokens);
        if let Some(cached) = self.cache.get(&key) {
            if cached.is_valid() {
                return Some(cached.response.clone());
            }
        }
        None
    }
    
    /// Insert a response into the cache
    pub fn insert(&mut self, prompt: &str, max_tokens: Option<u32>, response: String) {
        let key = hash_prompt(prompt, max_tokens);
        
        // If this key already exists, remove it first
        if self.cache.contains_key(&key) {
            self.remove_entry(key);
        }
        
        // Approximate memory usage of this response (very rough)
        let response_size = response.len();
        let prompt_size = prompt.len();
        let entry_size = response_size + prompt_size + 64; // Extra for overhead
        
        // If we're going to exceed the memory limit, clean up
        if self.estimated_memory_usage + entry_size > self.max_memory_usage {
            debug!("Cache memory limit reached, cleaning up");
            self.enforce_memory_limit(entry_size);
        }
        
        // Ensure we don't exceed max size
        if self.cache.len() >= self.max_size {
            // Remove the least recently used entry
            if let Some(oldest_key) = self.keys_queue.pop_front() {
                self.remove_entry(oldest_key);
            }
        }
        
        // Add the new entry
        let cached = CachedResponse::new(response);
        self.cache.insert(key, cached);
        self.keys_queue.push_back(key);
        self.estimated_memory_usage += entry_size;
        
        debug!("Added response to cache. Current size: {} items, ~{} MB", 
               self.cache.len(), 
               self.estimated_memory_usage / (1024 * 1024));
    }
    
    /// Remove an entry from the cache
    fn remove_entry(&mut self, key: u64) {
        if let Some(removed) = self.cache.remove(&key) {
            // Approximate the memory freed
            let freed_memory = removed.response.len() + 64; // Response + overhead
            self.estimated_memory_usage = self.estimated_memory_usage.saturating_sub(freed_memory);
        }
        
        // Also remove from the keys queue
        if let Some(pos) = self.keys_queue.iter().position(|&k| k == key) {
            self.keys_queue.remove(pos);
        }
    }
    
    /// Enforce memory limit by removing entries until we're under the limit
    fn enforce_memory_limit(&mut self, needed_space: usize) {
        while !self.keys_queue.is_empty() && self.estimated_memory_usage + needed_space > self.max_memory_usage {
            if let Some(oldest_key) = self.keys_queue.pop_front() {
                self.remove_entry(oldest_key);
            }
        }
    }
    
    /// Get the current size of the cache
    pub fn size(&self) -> usize {
        self.cache.len()
    }
    
    /// Get the current memory usage estimation in bytes
    pub fn memory_usage(&self) -> usize {
        self.estimated_memory_usage
    }
    
    /// Get the maximum allowed memory usage in bytes
    pub fn max_memory_usage(&self) -> usize {
        self.max_memory_usage
    }
    
    /// Clear expired entries from the cache
    pub fn clean(&mut self) -> usize {
        // Find expired entries
        let expired_keys: Vec<u64> = self.cache
            .iter()
            .filter(|(_, cached)| !cached.is_valid())
            .map(|(&key, _)| key)
            .collect();
        
        let count = expired_keys.len();
        
        for key in &expired_keys {
            self.remove_entry(*key);
        }
        
        if count > 0 {
            debug!("Cleaned {} expired entries from cache", count);
        }
        
        count
    }
}

/// An AI client wrapper that caches responses
pub struct CachedAiClient {
    /// The inner AI client that does the actual work
    inner: Box<dyn AiClient + Send + Sync>,
}

impl CachedAiClient {
    /// Create a new cached AI client that wraps another client
    pub fn new(inner: Box<dyn AiClient>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl AiClient for CachedAiClient {
    fn model_version(&self) -> &str {
        self.inner.model_version()
    }

    fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        // Check if we have a cached response
        let cache_read = RESPONSE_CACHE.read().await;
        if let Some(cached_response) = cache_read.get(prompt, None) {
            info!("Using cached response for prompt");
            return Ok(cached_response);
        }
        drop(cache_read); // Drop the read lock before acquiring write lock
        
        // Generate a new response
        let response = self.inner.generate(prompt).await?;
        
        // Cache the response
        let mut cache = RESPONSE_CACHE.write().await;
        cache.insert(prompt, None, response.clone());
        
        // Periodically clean the cache (every ~100 requests)
        if rand::random::<u8>() < 3 {  // ~1% chance
            debug!("Performing routine cache cleanup");
            cache.clean();
        }
        
        Ok(response)
    }
    
    async fn generate_with_options(&self, prompt: &str, options: RequestOptions) -> Result<String> {
        // Extract max_tokens for caching
        let max_tokens = options.max_tokens;
        
        // Check if the response is in the cache - use write lock to allow mutation
        let mut cache = RESPONSE_CACHE.write().await;
        if let Some(cached_response) = cache.get(prompt, max_tokens) {
            info!("Using cached response for prompt with max_tokens: {:?}", max_tokens);
            return Ok(cached_response);
        }
        
        // Not in cache, generate a new response
        let response = self.inner.generate_with_options(prompt, options).await?;
        
        // Cache the response - already have write lock
        cache.insert(prompt, max_tokens, response.clone());
        
        // Periodically clean the cache (every ~100 requests)
        if rand::random::<u8>() < 3 {  // ~1% chance
            debug!("Performing routine cache cleanup");
            cache.clean();
        }
        
        Ok(response)
    }
    
    async fn generate_streaming(&self, prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // For streaming responses, we can't easily cache the interim results
        // but we can still check if we have the full response cached
        let cache_read = RESPONSE_CACHE.read().await;
        if let Some(cached_response) = cache_read.get(prompt, None) {
            info!("Using cached response for streaming prompt");
            return Ok(Box::pin(futures::stream::once(async move { Ok(cached_response) })));
        }
        drop(cache_read); // Drop the read lock
        
        // Get a streaming response from the inner client
        let stream = self.inner.generate_streaming(prompt).await?;
        
        // Create a cloned prompt to move into the async block
        let prompt_clone = prompt.to_string();
        
        // Collect the full response while streaming
        let collected_stream = Box::pin(
            futures::stream::unfold(
                (stream, String::new()),
                move |(mut stream, mut collected)| {
                    let prompt_for_closure = prompt_clone.clone();
                    async move {
                        match stream.next().await {
                            Some(Ok(chunk)) => {
                                collected.push_str(&chunk);
                                Some((Ok(chunk), (stream, collected)))
                            }
                            Some(Err(e)) => Some((Err(e), (stream, collected))),
                            None => {
                                // Cache the complete response when done
                                if !collected.is_empty() {
                                    if let Ok(mut cache) = RESPONSE_CACHE.try_write() {
                                        cache.insert(&prompt_for_closure, None, collected);
                                    }
                                }
                                None
                            }
                        }
                    }
                },
            ),
        );
        
        Ok(collected_stream)
    }
    
    async fn generate_streaming_with_options(
        &self,
        prompt: &str,
        options: super::RequestOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let max_tokens = options.max_tokens;
        
        // For streaming responses, we can't easily cache the interim results
        // but we can still check if we have the full response cached
        let cache_read = RESPONSE_CACHE.read().await;
        if let Some(cached_response) = cache_read.get(prompt, max_tokens) {
            info!("Using cached response for streaming prompt with max_tokens: {:?}", max_tokens);
            return Ok(Box::pin(futures::stream::once(async move { Ok(cached_response) })));
        }
        drop(cache_read); // Drop the read lock
        
        // Get a streaming response from the inner client
        let stream = self.inner.generate_streaming_with_options(prompt, options).await?;
        
        // Create cloned parameters to move into the async block
        let prompt_clone = prompt.to_string();
        let max_tokens_clone = max_tokens;
        
        // Collect the full response while streaming
        let collected_stream = Box::pin(
            futures::stream::unfold(
                (stream, String::new()),
                move |(mut stream, mut collected)| {
                    let prompt_for_closure = prompt_clone.clone();
                    let max_tokens_for_closure = max_tokens_clone;
                    async move {
                        match stream.next().await {
                            Some(Ok(chunk)) => {
                                collected.push_str(&chunk);
                                Some((Ok(chunk), (stream, collected)))
                            }
                            Some(Err(e)) => Some((Err(e), (stream, collected))),
                            None => {
                                // Cache the complete response when done
                                if !collected.is_empty() {
                                    if let Ok(mut cache) = RESPONSE_CACHE.try_write() {
                                        cache.insert(&prompt_for_closure, max_tokens_for_closure, collected);
                                    }
                                }
                                None
                            }
                        }
                    }
                },
            ),
        );
        
        Ok(collected_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::RwLock;
    use std::sync::Arc;
    use futures::stream;
    use crate::ai::RequestOptions;

    // Manual mock implementation for testing
    struct MockAiClient {
        generate_response: Mutex<Option<String>>,
        generate_error: Mutex<Option<ToolkitError>>,
        generate_with_options_response: Mutex<Option<String>>,
        generate_with_options_error: Mutex<Option<ToolkitError>>,
    }

    impl MockAiClient {
        fn new() -> Self {
            Self {
                generate_response: Mutex::new(None),
                generate_error: Mutex::new(None),
                generate_with_options_response: Mutex::new(None),
                generate_with_options_error: Mutex::new(None),
            }
        }

        fn expect_generate(&self, result: Result<String>) {
            match result {
                Ok(response) => {
                    *self.generate_response.lock().unwrap() = Some(response);
                    *self.generate_error.lock().unwrap() = None;
                },
                Err(error) => {
                    *self.generate_response.lock().unwrap() = None;
                    *self.generate_error.lock().unwrap() = Some(error);
                }
            }
        }

        fn expect_generate_with_options(&self, result: Result<String>) {
            match result {
                Ok(response) => {
                    *self.generate_with_options_response.lock().unwrap() = Some(response);
                    *self.generate_with_options_error.lock().unwrap() = None;
                },
                Err(error) => {
                    *self.generate_with_options_response.lock().unwrap() = None;
                    *self.generate_with_options_error.lock().unwrap() = Some(error);
                }
            }
        }
    }

    #[async_trait]
    impl super::AiClient for MockAiClient {
        fn model_version(&self) -> &str {
            "mock-model"
        }

        fn base_url(&self) -> &str {
            "https://mock-api.example.com"
        }

        async fn generate(&self, _prompt: &str) -> Result<String> {
            let error_guard = self.generate_error.lock().unwrap();
            if let Some(error) = &*error_guard {
                return Err(error.clone());
            }
            drop(error_guard);
            
            let response_guard = self.generate_response.lock().unwrap();
            if let Some(response) = &*response_guard {
                Ok(response.clone())
            } else {
                Err(ToolkitError::Api("No mock response configured".to_string()))
            }
        }
        
        async fn generate_with_options(&self, _prompt: &str, _options: RequestOptions) -> Result<String> {
            let error_guard = self.generate_with_options_error.lock().unwrap();
            if let Some(error) = &*error_guard {
                return Err(error.clone());
            }
            drop(error_guard);
            
            let response_guard = self.generate_with_options_response.lock().unwrap();
            if let Some(response) = &*response_guard {
                Ok(response.clone())
            } else {
                Err(ToolkitError::Api("No mock response configured".to_string()))
            }
        }
        
        async fn generate_streaming(&self, _prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
            let result = self.generate(_prompt).await?;
            Ok(Box::pin(stream::once(async move { Ok(result) })))
        }
        
        async fn generate_streaming_with_options(
            &self,
            _prompt: &str,
            _options: RequestOptions,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
            let result = self.generate_with_options(_prompt, _options).await?;
            Ok(Box::pin(stream::once(async move { Ok(result) })))
        }
    }

    #[test]
    fn test_response_cache_basic_operations() {
        let mut cache = ResponseCache::new();
        let prompt = "test prompt";
        let response = "test response";
        
        // Test cache miss
        assert!(cache.get(prompt, None).is_none());
        
        // Test cache insert and hit
        cache.insert(prompt, None, response.to_string());
        assert_eq!(cache.get(prompt, None).unwrap(), response);
        
        // Test different max_tokens creates different cache entries
        cache.insert(prompt, Some(100), "different response".to_string());
        assert_eq!(cache.get(prompt, None).unwrap(), response);
        assert_eq!(cache.get(prompt, Some(100)).unwrap(), "different response");
    }

    #[test]
    fn test_response_cache_ttl() {
        let mut cache = ResponseCache::new();
        let prompt = "test prompt";
        let response = "test response";
        
        cache.insert(prompt, None, response.to_string());
        assert!(cache.get(prompt, None).is_some());
        
        // Simulate time passing
        let cached = cache.cache.get_mut(&hash_prompt(prompt, None)).unwrap();
        cached.cached_at = Instant::now() - CACHE_TTL - Duration::from_secs(1);
        
        // Should be expired now
        assert!(cache.get(prompt, None).is_none());
    }

    #[test]
    fn test_response_cache_clean() {
        let mut cache = ResponseCache::new();
        let prompt1 = "test prompt 1";
        let prompt2 = "test prompt 2";
        
        cache.insert(prompt1, None, "response 1".to_string());
        cache.insert(prompt2, None, "response 2".to_string());
        
        // Expire the first entry
        let cached = cache.cache.get_mut(&hash_prompt(prompt1, None)).unwrap();
        cached.cached_at = Instant::now() - CACHE_TTL - Duration::from_secs(1);
        
        // Clean should remove expired entries
        cache.clean();
        assert!(cache.get(prompt1, None).is_none());
        assert!(cache.get(prompt2, None).is_some());
    }

    #[tokio::test]
    async fn test_cached_ai_client_basic() {
        // Create a mock
        let mock = MockAiClient::new();
        mock.expect_generate(Ok("test response".to_string()));
        
        let client = CachedAiClient::new(Box::new(mock));
        
        // First call should hit the AI
        let response1 = client.generate("test prompt").await.unwrap();
        assert_eq!(response1, "test response");
        
        // Second call should hit the cache
        let response2 = client.generate("test prompt").await.unwrap();
        assert_eq!(response2, "test response");
    }

    #[tokio::test]
    async fn test_cached_ai_client_with_options() {
        // Create a mock
        let mock = MockAiClient::new();
        mock.expect_generate_with_options(Ok("test response with options".to_string()));
        
        // Create the cached client
        let client = CachedAiClient::new(Box::new(mock));
        
        // First call should hit the AI
        let response1 = client.generate_with_options("test prompt", crate::ai::RequestOptions { max_tokens: Some(100), ..Default::default() }).await.unwrap();
        assert_eq!(response1, "test response with options");
        
        // Second call should hit the cache
        let response2 = client.generate_with_options("test prompt", crate::ai::RequestOptions { max_tokens: Some(100), ..Default::default() }).await.unwrap();
        assert_eq!(response2, "test response with options");
    }

    #[tokio::test]
    async fn test_cached_ai_client_streaming() {
        // Create a mock
        let mock = MockAiClient::new();
        mock.expect_generate(Ok("test response".to_string()));
        
        let client = CachedAiClient::new(Box::new(mock));
        
        // First call should stream from AI
        let mut stream = client.generate_streaming("test prompt").await.unwrap();
        let response1 = stream.next().await.unwrap().unwrap();
        assert_eq!(response1, "test response");
        
        // Second call should hit the cache (no streaming)
        let mut stream = client.generate_streaming("test prompt").await.unwrap();
        let response2 = stream.next().await.unwrap().unwrap();
        assert_eq!(response2, "test response");
    }

    #[tokio::test]
    async fn test_error_propagation() {
        // Create a mock that returns an error
        let mock = MockAiClient::new();
        mock.expect_generate(Err(ToolkitError::Api("API error".to_string())));
        
        let client = CachedAiClient::new(Box::new(mock));
        
        // Error should be propagated
        let result = client.generate("error prompt").await;
        assert!(result.is_err());
        
        if let Err(ToolkitError::Api(msg)) = result {
            assert_eq!(msg, "API error");
        } else {
            panic!("Expected API error");
        }
    }

    #[tokio::test]
    async fn test_concurrent_cache_access() {
        let cache = RESPONSE_CACHE.clone();
        let prompt = "concurrent test";
        let response = "concurrent response";
        
        // Multiple writers
        let mut handles = vec![];
        for i in 0..5 {
            let cache = cache.clone();
            let prompt = format!("{} {}", prompt, i);
            let response = format!("{} {}", response, i);
            
            handles.push(tokio::spawn(async move {
                let mut cache = cache.write().await;
                cache.insert(&prompt, None, response);
            }));
        }
        
        // Wait for all writes
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all writes succeeded
        let cache_read = cache.read().await;
        for i in 0..5 {
            let prompt = format!("{} {}", prompt, i);
            let response = format!("{} {}", response, i);
            assert_eq!(cache_read.get(&prompt, None).unwrap(), response);
        }
    }
} 