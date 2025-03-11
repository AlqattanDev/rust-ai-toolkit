// examples/rate_limit_handling.rs
//
// This example demonstrates proper handling of rate limits and backoff strategies:
// - Configuring rate limits
// - Handling rate limit errors
// - Implementing exponential backoff
// - Monitoring request rates
//
// To run this example:
// 1. Make sure you have configured your AI provider (run `rust-ai-toolkit config` first)
// 2. Run: cargo run --example rate_limit_handling

use dotenv::dotenv;
use rust_ai_toolkit::ai::{AiClient, RequestOptions};
use rust_ai_toolkit::error::{Result, ToolkitError};
use rust_ai_toolkit::utils::rate_limiter;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use log::{info, warn, error};

// Number of requests to make in the example
const NUM_REQUESTS: usize = 15;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    dotenv().ok();
    
    // Set up logging with more verbose output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    
    println!("Rust AI Toolkit - Rate Limit Handling Example");
    println!("============================================");
    
    // Step 1: Get an AI client
    println!("\nInitializing AI client...");
    let client = rust_ai_toolkit::ai::get_client().await?;
    
    let provider = client.base_url().split('.').next().unwrap_or("unknown");
    println!("Using AI provider: {} with model: {}", provider, client.model_version());
    
    // Step 2: Configure rate limits
    // In a real application, you would configure this based on your provider's limits
    println!("\nConfiguring rate limits...");
    
    // For demonstration purposes, we'll use a very low rate limit
    // This will force us to handle rate limiting in our example
    let requests_per_minute = 5;
    println!("Setting rate limit to {} requests per minute", requests_per_minute);
    
    // Step 3: Define a simple prompt for our requests
    let prompt = "Write a single sentence about artificial intelligence.";
    
    // Step 4: Make multiple requests with rate limit handling
    println!("\nMaking {} requests with rate limit handling...", NUM_REQUESTS);
    println!("Watch for backoff and retry behavior when rate limits are hit.\n");
    
    let start_time = Instant::now();
    let mut successful_requests = 0;
    let mut rate_limited_requests = 0;
    
    for i in 1..=NUM_REQUESTS {
        println!("Request {}/{}:", i, NUM_REQUESTS);
        
        // Check if we can make a request according to our rate limiter
        if rate_limiter::can_make_request(provider) {
            // Record the request attempt
            rate_limiter::record_request(provider);
            
            println!("  Making request...");
            
            // Make the request
            match make_request_with_retry(&client, prompt, 3).await {
                Ok(response) => {
                    successful_requests += 1;
                    println!("  Success: {}", response);
                    
                    // Record successful request
                    rate_limiter::record_success(provider);
                }
                Err(e) => {
                    if is_rate_limit_error(&e) {
                        rate_limited_requests += 1;
                        println!("  Rate limit exceeded, backing off...");
                        
                        // Get the backoff time from the rate limiter
                        let backoff_ms = rate_limiter::record_failure(provider);
                        println!("  Backing off for {}ms", backoff_ms);
                        
                        // Wait for the backoff period
                        sleep(Duration::from_millis(backoff_ms)).await;
                    } else {
                        // For other errors, just report them
                        println!("  Error: {}", e);
                    }
                }
            }
        } else {
            // We're at our rate limit, need to wait
            println!("  Rate limit reached, waiting before next request...");
            
            // Wait for a bit before trying again
            // In a real application, you might use a more sophisticated approach
            sleep(Duration::from_secs(60 / requests_per_minute as u64 + 1)).await;
        }
        
        // Add a small delay between requests to make the output more readable
        sleep(Duration::from_millis(500)).await;
    }
    
    let elapsed = start_time.elapsed();
    
    // Step 5: Report results
    println!("\nResults:");
    println!("  Total requests attempted: {}", NUM_REQUESTS);
    println!("  Successful requests: {}", successful_requests);
    println!("  Rate limited requests: {}", rate_limited_requests);
    println!("  Total time: {:.2} seconds", elapsed.as_secs_f32());
    println!("  Average time per request: {:.2} seconds", elapsed.as_secs_f32() / NUM_REQUESTS as f32);
    
    // Step 6: Demonstrate a more robust approach with concurrent requests
    println!("\nDemonstrating concurrent requests with rate limiting...");
    
    // Create multiple concurrent tasks
    let concurrent_requests = 5;
    println!("Launching {} concurrent requests", concurrent_requests);
    
    let start_time = Instant::now();
    
    // Create a vector to hold our tasks
    let mut tasks = Vec::new();
    
    // Launch concurrent tasks
    for i in 1..=concurrent_requests {
        let client_clone = client.clone();
        let prompt = format!("Write a single sentence about topic {}.", i);
        
        // Spawn a new task for each request
        let task = tokio::spawn(async move {
            let task_id = i;
            println!("Task {} started", task_id);
            
            // Implement rate limiting and backoff within each task
            let mut attempts = 0;
            let max_attempts = 5;
            
            while attempts < max_attempts {
                attempts += 1;
                
                // Check if we can make a request
                if rate_limiter::can_make_request(provider) {
                    // Record the request
                    rate_limiter::record_request(provider);
                    
                    // Make the request
                    match client_clone.generate(&prompt).await {
                        Ok(response) => {
                            // Record success
                            rate_limiter::record_success(provider);
                            println!("Task {} succeeded on attempt {}", task_id, attempts);
                            return Ok(response);
                        }
                        Err(e) => {
                            if is_rate_limit_error(&e) {
                                // Get backoff time
                                let backoff_ms = rate_limiter::record_failure(provider);
                                println!("Task {} hit rate limit, backing off for {}ms", task_id, backoff_ms);
                                
                                // Wait for the backoff period
                                sleep(Duration::from_millis(backoff_ms)).await;
                            } else {
                                // For other errors, just return the error
                                return Err(e);
                            }
                        }
                    }
                } else {
                    // We're at our rate limit, need to wait
                    println!("Task {} waiting for rate limit window", task_id);
                    sleep(Duration::from_secs(12 / requests_per_minute as u64)).await;
                }
            }
            
            // If we've reached here, we've exceeded our max attempts
            Err(ToolkitError::RateLimit("Exceeded maximum retry attempts".to_string()))
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut successful_concurrent = 0;
    
    for task in tasks {
        match task.await {
            Ok(result) => {
                match result {
                    Ok(_) => successful_concurrent += 1,
                    Err(e) => println!("Task error: {}", e),
                }
            }
            Err(e) => println!("Task join error: {}", e),
        }
    }
    
    let elapsed = start_time.elapsed();
    
    println!("\nConcurrent request results:");
    println!("  Total concurrent tasks: {}", concurrent_requests);
    println!("  Successful tasks: {}", successful_concurrent);
    println!("  Total time: {:.2} seconds", elapsed.as_secs_f32());
    
    println!("\nRate limit handling example completed successfully!");
    
    Ok(())
}

// Helper function to make a request with retry logic
async fn make_request_with_retry(client: &dyn AiClient, prompt: &str, max_retries: usize) -> Result<String> {
    let mut attempts = 0;
    let mut last_error = None;
    
    // Exponential backoff parameters
    let initial_backoff_ms = 1000; // 1 second
    let max_backoff_ms = 32000;    // 32 seconds
    let backoff_factor = 2.0;
    
    while attempts < max_retries {
        attempts += 1;
        
        match client.generate(prompt).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                // Check if this is a rate limit error
                if is_rate_limit_error(&e) {
                    // Calculate backoff time with exponential backoff
                    let backoff_ms = (initial_backoff_ms as f64 * backoff_factor.powi(attempts as i32 - 1)) as u64;
                    let backoff_ms = backoff_ms.min(max_backoff_ms);
                    
                    println!("  Rate limit error on attempt {}, backing off for {}ms", attempts, backoff_ms);
                    
                    // Wait for the backoff period
                    sleep(Duration::from_millis(backoff_ms)).await;
                    
                    // Store the error and continue
                    last_error = Some(e);
                } else {
                    // For other errors, return immediately
                    return Err(e);
                }
            }
        }
    }
    
    // If we've reached here, we've exceeded our retry limit
    Err(last_error.unwrap_or_else(|| {
        ToolkitError::RateLimit("Exceeded maximum retry attempts".to_string())
    }))
}

// Helper function to check if an error is a rate limit error
fn is_rate_limit_error(error: &ToolkitError) -> bool {
    match error {
        ToolkitError::RateLimit(_) => true,
        ToolkitError::Api(msg) => msg.contains("rate limit") || msg.contains("429"),
        _ => false,
    }
} 