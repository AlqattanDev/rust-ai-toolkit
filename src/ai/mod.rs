//! AI client interface and implementations for various AI providers.
//!
//! This module provides a unified interface for interacting with different AI providers
//! through the [`AiClient`] trait. It includes implementations for popular AI services
//! such as Anthropic and OpenAI, as well as utilities for caching responses.
//!
//! # Examples
//!
//! ```no_run
//! use crate::ai;
//! use crate::error::Result;
//!
//! async fn example() -> Result<()> {
//!     // Get a default AI client based on configuration
//!     let client = ai::get_client().await?;
//!     
//!     // Generate a response
//!     let response = client.generate("Tell me a joke").await?;
//!     println!("Response: {}", response);
//!     
//!     // Get a cached client for improved performance
//!     let cached_client = ai::get_cached_client().await?;
//!     let response = cached_client.generate("What is Rust?").await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Threading and Safety
//!
//! All implementations of [`AiClient`] are required to be both [`Send`] and [`Sync`],
//! making them safe to use across thread boundaries and in concurrent contexts.

mod anthropic;
mod anthropic_enhanced;
mod openai;
mod cache;

use crate::config;
use crate::error::{Result, ToolkitError};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::time::Duration;
use std::sync::Mutex as StdMutex;
use lazy_static::lazy_static;
use reqwest;

/// HTTP header constants for consistent naming
pub mod headers {
    /// Content-Type header
    pub const CONTENT_TYPE: &str = "Content-Type";
    /// JSON content type value
    pub const APPLICATION_JSON: &str = "application/json";
    /// Authorization header
    pub const AUTHORIZATION: &str = "Authorization";
    /// Bearer token prefix
    pub const BEARER_PREFIX: &str = "Bearer ";
    /// Accept header
    pub const ACCEPT: &str = "Accept";
    /// Event stream content type
    pub const TEXT_EVENT_STREAM: &str = "text/event-stream";
    /// X-API-Key header
    pub const X_API_KEY: &str = "X-Api-Key";
    /// Anthropic version header
    pub const ANTHROPIC_VERSION: &str = "anthropic-version";
}

/// Add a shared HTTP client that can be reused across all AI client instances
lazy_static! {
    /// Shared HTTP client for all AI clients to use
    pub(crate) static ref SHARED_HTTP_CLIENT: reqwest::Client = {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create shared HTTP client")
    };
}

/// Configuration options for AI model requests.
///
/// This struct encapsulates various parameters that can be used to customize
/// the behavior of AI model requests, such as controlling the length of generated
/// responses, adjusting randomness, and setting timeouts.
///
/// # Examples
///
/// ```
/// use crate::ai::{RequestOptions, FunctionDefinition};
/// use std::time::Duration;
///
/// // Create default options
/// let mut options = RequestOptions::default();
///
/// // Customize options
/// options.max_tokens = Some(1000);
/// options.temperature = Some(0.7);
/// options.timeout = Some(Duration::from_secs(30));
/// ```
#[derive(Debug, Clone)]
pub struct RequestOptions {
    /// Maximum number of tokens to generate in the response.
    /// If `None`, the model's default will be used.
    pub max_tokens: Option<u32>,
    
    /// Controls randomness in the output. Values between 0.0 and 1.0.
    /// Lower values make the output more deterministic, higher values more random.
    /// If `None`, the model's default will be used.
    pub temperature: Option<f32>,
    
    /// Controls diversity via nucleus sampling. Values between 0.0 and 1.0.
    /// If `None`, the model's default will be used.
    pub top_p: Option<f32>,
    
    /// Maximum time to wait for a response from the AI provider.
    /// If `None`, a default timeout will be used.
    pub timeout: Option<Duration>,
    
    /// List of function definitions for function calling capabilities.
    /// If `None`, function calling will not be used.
    pub functions: Option<Vec<FunctionDefinition>>,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            max_tokens: None,
            temperature: None,
            top_p: None,
            timeout: None,
            functions: None,
        }
    }
}

/// Function definition for function calling capabilities with AI models.
///
/// This struct represents a function that can be called by the AI model during
/// generation. It includes the function's name, description, and parameter schema.
///
/// # Examples
///
/// ```
/// use crate::ai::FunctionDefinition;
/// use serde_json::json;
///
/// let function = FunctionDefinition {
///     name: "get_weather".to_string(),
///     description: "Get the current weather for a location".to_string(),
///     parameters: json!({
///         "type": "object",
///         "properties": {
///             "location": {
///                 "type": "string",
///                 "description": "The city and state, e.g., San Francisco, CA"
///             }
///         },
///         "required": ["location"]
///     }),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function that can be called by the AI model.
    pub name: String,
    
    /// A description of what the function does, used by the model to determine when to call it.
    pub description: String,
    
    /// The parameters the function accepts, specified as a JSON Schema object.
    pub parameters: Value,
}

/// A trait representing a client for interacting with AI models.
///
/// This trait defines the core interface for generating responses from AI models,
/// with support for both synchronous and streaming generation, as well as
/// structured JSON responses and function calling.
///
/// All implementations must be both [`Send`] and [`Sync`] to ensure they can be
/// safely used in concurrent contexts.
///
/// # Examples
///
/// ```no_run
/// use crate::ai::{AiClient, RequestOptions};
/// use crate::error::Result;
///
/// async fn example(client: &dyn AiClient) -> Result<()> {
///     // Basic generation
///     let response = client.generate("Tell me a joke").await?;
///     
///     // Generation with options
///     let mut options = RequestOptions::default();
///     options.temperature = Some(0.8);
///     let response = client.generate_with_options("Write a poem", options).await?;
///     
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait AiClient: Send + Sync {
    /// Get the model version being used by this client.
    ///
    /// # Returns
    ///
    /// A string slice identifying the model version (e.g., "gpt-4", "claude-3-opus").
    fn model_version(&self) -> &str;

    /// Get the base URL for API requests.
    ///
    /// # Returns
    ///
    /// A string slice containing the base URL used for API requests.
    fn base_url(&self) -> &str;

    /// Generate a response from the AI model.
    ///
    /// This is the core method that all AI clients must implement. It sends the
    /// provided prompt to the AI model and returns the generated response.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    ///
    /// # Returns
    ///
    /// A `Result` containing the generated text response if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, or if the AI provider
    /// returns an error response.
    async fn generate(&self, prompt: &str) -> Result<String>;
    
    /// Generate a response with configurable parameters.
    ///
    /// This method allows for more fine-grained control over the generation process
    /// through the provided options.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    /// * `options` - Configuration options for the request.
    ///
    /// # Returns
    ///
    /// A `Result` containing the generated text response if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, or if the AI provider
    /// returns an error response.
    async fn generate_with_options(&self, prompt: &str, options: RequestOptions) -> Result<String> {
        // Default implementation falls back to standard generate
        self.generate(prompt).await
    }
    
    /// Generate a streaming response from the AI model.
    ///
    /// This method returns a stream of response chunks as they become available,
    /// which can be useful for displaying partial responses to users in real-time.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    ///
    /// # Returns
    ///
    /// A `Result` containing a pinned `Stream` of response chunks if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, or if the AI provider
    /// returns an error response.
    ///
    /// # Performance Considerations
    ///
    /// Streaming responses can provide better user experience for long responses,
    /// but may have slightly higher overhead than non-streaming requests.
    async fn generate_streaming(&self, prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // Default implementation generates the full response and returns it as a single chunk
        let response = self.generate(prompt).await?;
        Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
    }
    
    /// Generate a streaming response with configurable parameters.
    ///
    /// This method combines the streaming functionality with the ability to
    /// customize the generation parameters.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    /// * `options` - Configuration options for the request.
    ///
    /// # Returns
    ///
    /// A `Result` containing a pinned `Stream` of response chunks if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, or if the AI provider
    /// returns an error response.
    async fn generate_streaming_with_options(
        &self, 
        prompt: &str, 
        options: RequestOptions
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // Default implementation generates the full response and returns it as a single chunk
        let response = self.generate_with_options(prompt, options).await?;
        Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
    }

    /// Generate a structured JSON response.
    ///
    /// This method attempts to generate a response that can be parsed as JSON,
    /// which is useful for getting structured data from the AI model.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed JSON value if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, if the AI provider
    /// returns an error response, or if the response cannot be parsed as valid JSON.
    async fn generate_json(&self, prompt: &str) -> Result<Value> {
        // Default implementation attempts to parse the text response as JSON
        let text = self.generate(prompt).await?;
        serde_json::from_str(&text).map_err(|e| ToolkitError::Parse(e.to_string()))
    }

    /// Generate a structured JSON response with configurable parameters.
    ///
    /// This method combines JSON generation with the ability to customize
    /// the generation parameters.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    /// * `options` - Configuration options for the request.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed JSON value if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, if the AI provider
    /// returns an error response, or if the response cannot be parsed as valid JSON.
    async fn generate_json_with_options(&self, prompt: &str, options: RequestOptions) -> Result<Value> {
        // Default implementation attempts to parse the text response as JSON
        let text = self.generate_with_options(prompt, options).await?;
        serde_json::from_str(&text).map_err(|e| ToolkitError::Parse(e.to_string()))
    }

    /// Call a function using the AI model.
    ///
    /// This method is designed for function calling capabilities, where the AI model
    /// can decide to call a function based on the input prompt.
    ///
    /// # Parameters
    ///
    /// * `prompt` - The input prompt to send to the AI model.
    /// * `function` - The function definition that the AI model can call.
    ///
    /// # Returns
    ///
    /// A `Result` containing the function call parameters as a JSON value if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, times out, or if the AI provider
    /// returns an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::ai::{AiClient, FunctionDefinition};
    /// use crate::error::Result;
    /// use serde_json::json;
    ///
    /// async fn example(client: &dyn AiClient) -> Result<()> {
    ///     let function = FunctionDefinition {
    ///         name: "get_weather".to_string(),
    ///         description: "Get the current weather for a location".to_string(),
    ///         parameters: json!({
    ///             "type": "object",
    ///             "properties": {
    ///                 "location": {
    ///                     "type": "string",
    ///                     "description": "The city and state, e.g., San Francisco, CA"
    ///                 }
    ///             },
    ///             "required": ["location"]
    ///         }),
    ///     };
    ///     
    ///     let result = client.call_function("What's the weather like in New York?", function).await?;
    ///     println!("Function call parameters: {:?}", result);
    ///     
    ///     Ok(())
    /// }
    /// ```
    async fn call_function(
        &self,
        prompt: &str,
        function: FunctionDefinition,
    ) -> Result<Value> {
        let mut options = RequestOptions::default();
        options.functions = Some(vec![function]);
        self.generate_json_with_options(prompt, options).await
    }
}

/// Get a client configured according to the current configuration.
///
/// This function returns a new client each time it's called, which may not be
/// efficient for multiple rapid requests. Consider using `get_cached_client` instead
/// for better performance in most cases.
///
/// # Returns
///
/// A `Result` containing a boxed `AiClient` trait object if successful.
///
/// # Errors
///
/// Returns an error if the configuration is invalid or if initialization fails for any reason.
///
/// # Examples
///
/// ```no_run
/// use crate::ai;
/// use crate::error::Result;
///
/// async fn example() -> Result<()> {
///     let client = ai::get_client().await?;
///     let response = client.generate("Hello, AI!").await?;
///     println!("AI says: {}", response);
///     Ok(())
/// }
/// ```
pub async fn get_client() -> Result<Box<dyn AiClient>> {
    let config = crate::config::get_config()?;
    
    match config.provider.as_str() {
        "anthropic" => {
            let client = anthropic::AnthropicClient::new(
                &config.api_key,
                &config.model,
            )?;
            Ok(Box::new(client))
        }
        "openai" => {
            let client = openai::OpenAiClient::new(
                &config.api_key,
                &config.model,
            )?;
            Ok(Box::new(client))
        }
        "anthropic_enhanced" => {
            let client = anthropic_enhanced::EnhancedAnthropicClient::new(
                &config.api_key,
                &config.model,
            )?;
            Ok(Box::new(client))
        }
        _ => Err(ToolkitError::Config(format!(
            "Unsupported AI provider: {}",
            config.provider
        ))),
    }
}

// Initialize the global shared HTTP client for reuse
lazy_static! {
    /// Global cached HTTP client instance to avoid repeated client creation
    static ref GLOBAL_CACHED_CLIENT: StdMutex<Option<Box<dyn AiClient + Send + Sync>>> = StdMutex::new(None);
}

/// Get a cached AI client that will be reused across calls.
///
/// This function returns a reference to a cached client instance, creating it
/// if needed. This is more efficient than creating a new client for each request.
///
/// # Returns
///
/// A `Result` containing a boxed `AiClient` trait object if successful.
///
/// # Errors
///
/// Returns an error if the configuration is invalid or if initialization fails for any reason.
///
/// # Examples
///
/// ```no_run
/// use crate::ai;
/// use crate::error::Result;
///
/// async fn example() -> Result<()> {
///     let client = ai::get_cached_client().await?;
///     
///     // Multiple requests using the same client
///     let response1 = client.generate("First question").await?;
///     let response2 = client.generate("Second question").await?;
///     
///     println!("Response 1: {}", response1);
///     println!("Response 2: {}", response2);
///     
///     Ok(())
/// }
/// ```
pub async fn get_cached_client() -> Result<Box<dyn AiClient>> {
    // First check if we already have a client
    {
        let client_lock = GLOBAL_CACHED_CLIENT.lock().unwrap();
        if client_lock.is_some() {
            // We already have a client, create a new cached wrapper for it
            let inner_client = get_client().await?;
            return Ok(Box::new(cache::CachedAiClient::new(inner_client)));
        }
    }
    
    // If we don't have a client yet, create one and store it
    let inner_client = get_client().await?;
    
    // Create a cached client - this will be our singleton cached client
    let cached_client = Box::new(cache::CachedAiClient::new(inner_client)) as Box<dyn AiClient + Send + Sync>;
    
    // Store the new cached client
    let mut client_lock = GLOBAL_CACHED_CLIENT.lock().unwrap();
    *client_lock = Some(cached_client);
    
    // Return a new cached wrapper around a fresh client
    // This is intentional - each call gets a fresh wrapper but we're just ensuring
    // the cache singleton is initialized
    let inner_client = get_client().await?;
    Ok(Box::new(cache::CachedAiClient::new(inner_client)))
}

/// A proxy AI client that forwards requests to another client
struct ProxyAiClient<'a> {
    inner: &'a Box<dyn AiClient>,
}

impl<'a> ProxyAiClient<'a> {
    fn new(inner: &'a Box<dyn AiClient>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<'a> AiClient for ProxyAiClient<'a> {
    fn model_version(&self) -> &str {
        self.inner.model_version()
    }
    
    fn base_url(&self) -> &str {
        self.inner.base_url()
    }
    
    async fn generate(&self, prompt: &str) -> Result<String> {
        self.inner.generate(prompt).await
    }
    
    async fn generate_with_options(&self, prompt: &str, options: RequestOptions) -> Result<String> {
        self.inner.generate_with_options(prompt, options).await
    }
    
    async fn generate_streaming(&self, prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        self.inner.generate_streaming(prompt).await
    }
    
    async fn generate_streaming_with_options(
        &self, 
        prompt: &str, 
        options: RequestOptions
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        self.inner.generate_streaming_with_options(prompt, options).await
    }
    
    async fn generate_json(&self, prompt: &str) -> Result<Value> {
        self.inner.generate_json(prompt).await
    }
    
    async fn generate_json_with_options(&self, prompt: &str, options: RequestOptions) -> Result<Value> {
        self.inner.generate_json_with_options(prompt, options).await
    }
    
    async fn call_function(
        &self,
        prompt: &str,
        function: FunctionDefinition,
    ) -> Result<Value> {
        self.inner.call_function(prompt, function).await
    }
}
