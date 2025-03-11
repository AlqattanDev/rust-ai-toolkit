//! Rate limiting functionality for API requests.
//!
//! This module provides rate limiting capabilities to prevent exceeding API provider
//! rate limits. It implements a sliding window approach to track requests over time
//! and includes exponential backoff for handling failures and rate limit responses.
//!
//! The main components are:
//! - [`Provider`]: Enum representing different API providers
//! - Public functions for checking and recording requests
//! - Internal rate limiting implementation with provider-specific limits
//!
//! # Examples
//!
//! ```
//! use crate::utils::rate_limiter;
//!
//! // Check if a request can be made
//! if rate_limiter::can_make_request("anthropic") {
//!     // Make the request
//!     // ...
//!
//!     // Record the request
//!     rate_limiter::record_request("anthropic");
//!
//!     // Record success or failure
//!     if success {
//!         rate_limiter::record_success("anthropic");
//!     } else {
//!         let backoff_ms = rate_limiter::record_failure("anthropic");
//!         // Wait for backoff_ms before retrying
//!     }
//! } else {
//!     // Cannot make request, rate limit exceeded
//! }
//! ```
//!
//! # Thread Safety
//!
//! The rate limiter uses a mutex to protect its internal state, making it safe
//! to use from multiple threads. However, if the mutex cannot be acquired, the
//! rate limiter will allow requests to proceed to avoid blocking the application.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use log::{warn, info};
use colored::Colorize;

/// Trait for clock abstraction to make testing easier.
///
/// This trait abstracts the system clock to allow for deterministic testing
/// of time-dependent functionality.
pub trait Clock: Send + Sync {
    /// Get the current time.
    ///
    /// # Returns
    ///
    /// The current time as an `Instant`.
    fn now(&self) -> Instant;
}

/// Real clock implementation.
///
/// This is the default clock implementation that uses the system clock.
#[derive(Default)]
struct RealClock;

impl Clock for RealClock {
    /// Get the current time from the system clock.
    ///
    /// # Returns
    ///
    /// The current system time as an `Instant`.
    fn now(&self) -> Instant {
        Instant::now()
    }
}

// Singleton rate limiter instance with configurable clock
static RATE_LIMITER: Lazy<Arc<Mutex<RateLimiter<RealClock>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(RateLimiter::new(RealClock::default())))
});

// Default rate limits (requests per minute)
const DEFAULT_RPM_LIMIT: u32 = 30;
const WARN_THRESHOLD_PERCENT: f32 = 0.8; // Warn at 80% of limit

// Default backoff settings
const INITIAL_RETRY_DELAY_MS: u64 = 1000; // 1 second
const MAX_RETRY_DELAY_MS: u64 = 60000;    // 1 minute
const BACKOFF_FACTOR: f32 = 2.0;

/// Represents an API provider for rate limiting.
///
/// This enum defines the supported API providers, each with their own
/// rate limits and configurations.
///
/// # Examples
///
/// ```
/// use crate::utils::rate_limiter::Provider;
///
/// // Create a provider from a string
/// let provider = Provider::from("anthropic");
/// assert_eq!(provider, Provider::Anthropic);
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Provider {
    /// Anthropic API provider (Claude models)
    Anthropic,
    /// OpenAI API provider (GPT models)
    OpenAI,
    /// Custom or unknown API provider
    Custom,
}

impl From<&str> for Provider {
    /// Convert a string to a Provider enum.
    ///
    /// This allows for easy conversion from configuration strings to the
    /// appropriate provider enum variant.
    ///
    /// # Parameters
    ///
    /// * `provider` - The provider name as a string.
    ///
    /// # Returns
    ///
    /// The corresponding `Provider` enum variant.
    fn from(provider: &str) -> Self {
        match provider {
            "anthropic" | "anthropic_enhanced" => Provider::Anthropic,
            "openai" => Provider::OpenAI,
            _ => Provider::Custom,
        }
    }
}

/// Tracks rate limiting information for a provider.
///
/// This struct maintains the state needed for rate limiting a specific provider,
/// including request history, limits, and backoff information.
#[derive(Debug)]
struct ProviderRateLimit {
    requests: Vec<Instant>,
    rpm_limit: u32,
    consecutive_failures: u32,
    last_backoff_delay_ms: u64,
}

impl ProviderRateLimit {
    /// Create a new provider rate limit with the specified RPM limit.
    ///
    /// # Parameters
    ///
    /// * `rpm_limit` - The maximum number of requests per minute allowed.
    ///
    /// # Returns
    ///
    /// A new `ProviderRateLimit` instance.
    fn new(rpm_limit: u32) -> Self {
        Self {
            requests: Vec::new(),
            rpm_limit,
            consecutive_failures: 0,
            last_backoff_delay_ms: INITIAL_RETRY_DELAY_MS,
        }
    }

    /// Remove requests older than one minute from the history.
    ///
    /// This method implements the sliding window approach by removing
    /// requests that are no longer relevant for rate limiting calculations.
    ///
    /// # Parameters
    ///
    /// * `clock` - The clock implementation to use for time calculations.
    fn cleanup_old_requests<C: Clock>(&mut self, clock: &C) {
        let one_minute_ago = clock.now() - Duration::from_secs(60);
        self.requests.retain(|time| *time > one_minute_ago);
    }

    /// Get the current requests per minute count.
    ///
    /// # Parameters
    ///
    /// * `clock` - The clock implementation to use for time calculations.
    ///
    /// # Returns
    ///
    /// The number of requests made in the last minute.
    fn get_current_rpm<C: Clock>(&mut self, clock: &C) -> u32 {
        self.cleanup_old_requests(clock);
        self.requests.len() as u32
    }

    /// Check if a request can be made without exceeding the rate limit.
    ///
    /// # Parameters
    ///
    /// * `clock` - The clock implementation to use for time calculations.
    ///
    /// # Returns
    ///
    /// `true` if a request can be made, `false` if the rate limit would be exceeded.
    fn can_make_request<C: Clock>(&mut self, clock: &C) -> bool {
        self.cleanup_old_requests(clock);
        self.requests.len() < self.rpm_limit as usize
    }

    /// Record a request in the history.
    ///
    /// # Parameters
    ///
    /// * `clock` - The clock implementation to use for time calculations.
    fn record_request<C: Clock>(&mut self, clock: &C) {
        self.requests.push(clock.now());
    }

    /// Record a successful request, resetting the failure count.
    fn record_success(&mut self) {
        // Reset failure count on success
        self.consecutive_failures = 0;
        self.last_backoff_delay_ms = INITIAL_RETRY_DELAY_MS;
    }

    /// Record a failed request and calculate the backoff delay.
    ///
    /// # Returns
    ///
    /// The backoff delay in milliseconds before the next retry.
    fn record_failure(&mut self) -> u64 {
        self.consecutive_failures += 1;
        
        // Calculate exponential backoff
        if self.consecutive_failures > 1 {
            self.last_backoff_delay_ms = (self.last_backoff_delay_ms as f32 * BACKOFF_FACTOR) as u64;
            if self.last_backoff_delay_ms > MAX_RETRY_DELAY_MS {
                self.last_backoff_delay_ms = MAX_RETRY_DELAY_MS;
            }
        }
        
        self.last_backoff_delay_ms
    }

    /// Records a rate limit response from the API.
    ///
    /// This is used when we receive a 429 Too Many Requests response.
    /// It increases the backoff delay more aggressively than a normal failure.
    fn record_rate_limit(&mut self) {
        // Increase the consecutive failures counter
        self.consecutive_failures += 1;
        
        // Calculate the appropriate backoff delay
        self.last_backoff_delay_ms = (INITIAL_RETRY_DELAY_MS as f32 * 
            BACKOFF_FACTOR.powi(self.consecutive_failures as i32))
            .min(MAX_RETRY_DELAY_MS as f32) as u64;
        
        warn!("Rate limit exceeded for provider. Backing off for {}ms", 
            self.last_backoff_delay_ms);
            
        // Don't remove any requests - we want the rate limiter to be cautious
    }
}

/// Main rate limiter implementation.
///
/// This struct manages rate limiting for multiple providers, tracking
/// request history and enforcing rate limits.
#[derive(Debug)]
pub struct RateLimiter<C: Clock> {
    providers: HashMap<Provider, ProviderRateLimit>,
    clock: C,
}

impl<C: Clock> RateLimiter<C> {
    /// Create a new rate limiter with the specified clock implementation.
    ///
    /// # Parameters
    ///
    /// * `clock` - The clock implementation to use for time calculations.
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance with default provider limits.
    fn new(clock: C) -> Self {
        let mut providers = HashMap::new();
        providers.insert(Provider::Anthropic, ProviderRateLimit::new(30)); // 30 RPM for Anthropic
        providers.insert(Provider::OpenAI, ProviderRateLimit::new(60));    // 60 RPM for OpenAI
        providers.insert(Provider::Custom, ProviderRateLimit::new(DEFAULT_RPM_LIMIT));
        
        Self { providers, clock }
    }
    
    /// Checks if a request can be made to the specified provider.
    ///
    /// This method also logs warnings if the rate limit is being approached.
    ///
    /// # Parameters
    ///
    /// * `provider` - The provider to check.
    ///
    /// # Returns
    ///
    /// `true` if a request can be made, `false` if the rate limit would be exceeded.
    fn check_rate_limit(&mut self, provider: Provider) -> bool {
        let rate_limit = self.providers
            .entry(provider)
            .or_insert_with(|| ProviderRateLimit::new(DEFAULT_RPM_LIMIT));
            
        // Check if we're approaching the limit and warn if so
        let current_rpm = rate_limit.get_current_rpm(&self.clock);
        let limit = rate_limit.rpm_limit;
        
        let usage_percent = current_rpm as f32 / limit as f32;
        if usage_percent >= WARN_THRESHOLD_PERCENT {
            warn!(
                "Approaching rate limit for {:?}: {}/{} requests ({}%)",
                provider, current_rpm, limit, (usage_percent * 100.0) as u32
            );
            
            println!("{}", format!(
                "Warning: Approaching API rate limit for {:?} ({}/{} requests, {}%)",
                provider, current_rpm, limit, (usage_percent * 100.0) as u32
            ).yellow());
        }
        
        rate_limit.can_make_request(&self.clock)
    }
    
    /// Records a successful request.
    ///
    /// # Parameters
    ///
    /// * `provider` - The provider to record the success for.
    fn record_success(&mut self, provider: Provider) {
        if let Some(rate_limit) = self.providers.get_mut(&provider) {
            rate_limit.record_success();
        }
    }
    
    /// Records a failed request and returns the delay before retry.
    ///
    /// # Parameters
    ///
    /// * `provider` - The provider to record the failure for.
    ///
    /// # Returns
    ///
    /// The backoff delay in milliseconds before the next retry.
    fn record_failure(&mut self, provider: Provider) -> u64 {
        if let Some(rate_limit) = self.providers.get_mut(&provider) {
            rate_limit.record_failure()
        } else {
            INITIAL_RETRY_DELAY_MS
        }
    }

    /// Records a rate limit response for a specific provider.
    ///
    /// # Parameters
    ///
    /// * `provider` - The provider that returned a rate limit response.
    fn record_rate_limit(&mut self, provider: Provider) {
        let provider_limits = self.providers
            .entry(provider)
            .or_insert_with(|| {
                let rpm = match provider {
                    Provider::Anthropic => 10, // Anthropic has lower limits
                    Provider::OpenAI => DEFAULT_RPM_LIMIT,
                    Provider::Custom => DEFAULT_RPM_LIMIT,
                };
                ProviderRateLimit::new(rpm)
            });
            
        provider_limits.record_rate_limit();
    }
}

// Public API

/// Check if a request can be made to the specified provider.
///
/// This function checks if making a request to the specified provider
/// would exceed its rate limit.
///
/// # Parameters
///
/// * `provider_str` - The provider name as a string.
///
/// # Returns
///
/// `true` if a request can be made, `false` if the rate limit would be exceeded.
///
/// # Examples
///
/// ```
/// use crate::utils::rate_limiter;
///
/// if rate_limiter::can_make_request("anthropic") {
///     // Make the request
/// } else {
///     // Wait or handle rate limit exceeded
/// }
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe. If the mutex cannot be acquired, it will
/// allow the request to proceed to avoid blocking the application.
pub fn can_make_request(provider_str: &str) -> bool {
    let provider = Provider::from(provider_str);
    if let Ok(mut limiter) = RATE_LIMITER.lock() {
        limiter.check_rate_limit(provider)
    } else {
        // If we can't acquire the lock, allow the request
        true
    }
}

/// Records a request to the specified provider.
///
/// This function should be called when a request is made to the specified
/// provider to update the rate limiting state.
///
/// # Parameters
///
/// * `provider_str` - The provider name as a string.
///
/// # Examples
///
/// ```
/// use crate::utils::rate_limiter;
///
/// // After making a request
/// rate_limiter::record_request("anthropic");
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe. If the mutex cannot be acquired, the
/// request will not be recorded, but this will not affect the application.
pub fn record_request(provider_str: &str) {
    let provider = Provider::from(provider_str);
    if let Ok(mut limiter) = RATE_LIMITER.lock() {
        // Get the current time before borrowing the rate limit
        let now = limiter.clock.now();
        
        if let Some(rate_limit) = limiter.providers.get_mut(&provider) {
            rate_limit.requests.push(now);
        }
    }
}

/// Records a successful request to the specified provider.
///
/// This function should be called when a request to the specified provider
/// completes successfully to reset the failure count.
///
/// # Parameters
///
/// * `provider_str` - The provider name as a string.
///
/// # Examples
///
/// ```
/// use crate::utils::rate_limiter;
///
/// // After a successful request
/// rate_limiter::record_success("anthropic");
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe. If the mutex cannot be acquired, the
/// success will not be recorded, but this will not affect the application.
pub fn record_success(provider_str: &str) {
    let provider = Provider::from(provider_str);
    if let Ok(mut limiter) = RATE_LIMITER.lock() {
        limiter.record_success(provider);
    }
}

/// Records a failed request and returns the delay before retry.
///
/// This function should be called when a request to the specified provider
/// fails to update the backoff state.
///
/// # Parameters
///
/// * `provider_str` - The provider name as a string.
///
/// # Returns
///
/// The backoff delay in milliseconds before the next retry.
///
/// # Examples
///
/// ```
/// use crate::utils::rate_limiter;
/// use std::thread;
/// use std::time::Duration;
///
/// // After a failed request
/// let backoff_ms = rate_limiter::record_failure("anthropic");
/// thread::sleep(Duration::from_millis(backoff_ms));
/// // Retry the request
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe. If the mutex cannot be acquired, a default
/// backoff delay will be returned.
pub fn record_failure(provider_str: &str) -> u64 {
    let provider = Provider::from(provider_str);
    if let Ok(mut limiter) = RATE_LIMITER.lock() {
        limiter.record_failure(provider)
    } else {
        INITIAL_RETRY_DELAY_MS
    }
}

/// Sets the rate limit for a provider (requests per minute)
pub fn set_rate_limit(provider_str: &str, rpm: u32) {
    let provider = Provider::from(provider_str);
    if let Ok(mut limiter) = RATE_LIMITER.lock() {
        if let Some(rate_limit) = limiter.providers.get_mut(&provider) {
            rate_limit.rpm_limit = rpm;
            info!("Rate limit for {:?} set to {} requests per minute", provider, rpm);
        }
    }
}

/// Records a rate limit response for a provider
/// This is used when we receive a 429 Too Many Requests response
pub fn record_rate_limit(provider_str: &str) {
    let provider = Provider::from(provider_str);
    
    let mut limiter = RATE_LIMITER.lock().unwrap();
    limiter.record_rate_limit(provider);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock clock for testing
    #[derive(Debug, Clone)]
    struct MockClock {
        now: Arc<AtomicU64>,
    }

    impl MockClock {
        fn new() -> Self {
            Self {
                now: Arc::new(AtomicU64::new(0)),
            }
        }

        fn advance(&self, duration: Duration) {
            self.now.fetch_add(duration.as_nanos() as u64, Ordering::SeqCst);
        }
    }

    impl Clock for MockClock {
        fn now(&self) -> Instant {
            let nanos = self.now.load(Ordering::SeqCst);
            // Convert our counter to an Instant by using a base instant and adding duration
            let base = Instant::now();
            base + Duration::from_nanos(nanos)
        }
    }

    #[test]
    fn test_request_counting() {
        let clock = MockClock::new();
        let mut limiter = RateLimiter::new(clock.clone());
        let provider = Provider::Custom;

        // Set a limit of 5 requests per minute
        limiter.providers.get_mut(&provider).unwrap().rpm_limit = 5;

        // Make 5 requests
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(provider));
            limiter.providers.get_mut(&provider).unwrap().record_request(&clock);
        }

        // 6th request should be denied
        assert!(!limiter.check_rate_limit(provider));

        // Advance time by 1 minute
        clock.advance(Duration::from_secs(60));

        // Should be able to make requests again
        assert!(limiter.check_rate_limit(provider));
    }

    #[test]
    fn test_rate_limit_detection() {
        let clock = MockClock::new();
        let mut limiter = RateLimiter::new(clock.clone());
        let provider = Provider::Custom;

        // Set a limit of 10 requests per minute
        limiter.providers.get_mut(&provider).unwrap().rpm_limit = 10;

        // Make 8 requests (80% of limit - should trigger warning)
        for _ in 0..8 {
            limiter.check_rate_limit(provider);
            limiter.providers.get_mut(&provider).unwrap().record_request(&clock);
        }

        // Make 2 more requests (should hit limit)
        for _ in 0..2 {
            limiter.check_rate_limit(provider);
            limiter.providers.get_mut(&provider).unwrap().record_request(&clock);
        }

        // Should be rate limited
        assert!(!limiter.check_rate_limit(provider));
    }

    #[test]
    fn test_exponential_backoff() {
        let clock = MockClock::new();
        let mut limiter = RateLimiter::new(clock);
        let provider = Provider::Custom;

        // Initial backoff should be 1 second
        assert_eq!(limiter.record_failure(provider), INITIAL_RETRY_DELAY_MS);

        // Second failure should double the backoff
        assert_eq!(limiter.record_failure(provider), INITIAL_RETRY_DELAY_MS * 2);

        // Third failure should double again
        assert_eq!(limiter.record_failure(provider), INITIAL_RETRY_DELAY_MS * 4);

        // Success should reset the backoff
        limiter.record_success(provider);
        assert_eq!(limiter.record_failure(provider), INITIAL_RETRY_DELAY_MS);
    }

    #[test]
    fn test_provider_specific_limits() {
        let clock = MockClock::new();
        let mut limiter = RateLimiter::new(clock.clone());

        // Test Anthropic limit (30 RPM)
        assert_eq!(limiter.providers[&Provider::Anthropic].rpm_limit, 30);
        
        // Test OpenAI limit (60 RPM)
        assert_eq!(limiter.providers[&Provider::OpenAI].rpm_limit, 60);

        // Test custom provider (default limit)
        assert_eq!(limiter.providers[&Provider::Custom].rpm_limit, DEFAULT_RPM_LIMIT);
    }

    #[test]
    fn test_concurrent_access() {
        let clock = MockClock::new();
        let limiter = Arc::new(Mutex::new(RateLimiter::new(clock.clone())));
        let provider = Provider::Custom;

        // Set a high limit for testing
        limiter.lock().unwrap().providers.get_mut(&provider).unwrap().rpm_limit = 1000;

        let mut handles = vec![];
        let request_count = Arc::new(AtomicU64::new(0));

        // Spawn 10 threads making requests
        for _ in 0..10 {
            let limiter = Arc::clone(&limiter);
            let clock = clock.clone();
            let request_count = Arc::clone(&request_count);

            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    let mut limiter = limiter.lock().unwrap();
                    if limiter.check_rate_limit(provider) {
                        limiter.providers.get_mut(&provider).unwrap().record_request(&clock);
                        request_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }));
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify the total number of requests
        let total_requests = request_count.load(Ordering::SeqCst);
        assert!(total_requests > 0);
        assert!(total_requests <= 1000); // Should not exceed the rate limit
    }
} 