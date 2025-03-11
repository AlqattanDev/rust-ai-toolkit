// examples/custom_ai_client.rs
//
// This example demonstrates how to implement a custom AI provider for the Rust AI Toolkit:
// - Creating a custom AI client that implements the AiClient trait
// - Handling requests and responses
// - Integrating with the toolkit's architecture
//
// To run this example:
// 1. Run: cargo run --example custom_ai_client

use async_trait::async_trait;
use dotenv::dotenv;
use futures::stream::{self, Stream};
use rust_ai_toolkit::ai::{AiClient, RequestOptions, FunctionDefinition};
use rust_ai_toolkit::error::{Result, ToolkitError};
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    dotenv().ok();
    
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("Rust AI Toolkit - Custom AI Client Example");
    println!("=========================================");
    
    // Step 1: Create our custom AI client
    println!("\nCreating custom AI client...");
    let client = Arc::new(MockAiClient::new("mock-gpt", "https://mock-ai-api.example.com"));
    
    println!("Custom client created:");
    println!("  Model: {}", client.model_version());
    println!("  Base URL: {}", client.base_url());
    
    // Step 2: Use the client for a basic request
    let prompt = "Tell me about artificial intelligence.";
    println!("\nSending prompt to custom AI client: \"{}\"", prompt);
    
    let response = client.generate(prompt).await?;
    println!("\nResponse from custom AI client:");
    println!("-----------------------------------");
    println!("{}", response);
    
    // Step 3: Test with custom options
    println!("\nTesting with custom options...");
    
    let options = RequestOptions {
        max_tokens: Some(100),
        temperature: Some(0.8),
        top_p: None,
        timeout: Some(Duration::from_secs(30)),
        functions: None,
    };
    
    let response = client.generate_with_options(prompt, options).await?;
    println!("\nResponse with custom options:");
    println!("-----------------------------------");
    println!("{}", response);
    
    // Step 4: Test streaming functionality
    println!("\nTesting streaming functionality...");
    
    let mut stream = client.generate_streaming(prompt).await?;
    
    println!("\nStreaming response:");
    println!("-----------------------------------");
    
    let mut full_response = String::new();
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_response.push_str(&chunk);
            }
            Err(e) => {
                eprintln!("\nError receiving chunk: {}", e);
                return Err(e);
            }
        }
    }
    
    println!("\n-----------------------------------");
    
    // Step 5: Test function calling
    println!("\nTesting function calling...");
    
    let function = FunctionDefinition {
        name: "get_weather".to_string(),
        description: "Get the current weather for a location".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g., San Francisco, CA"
                }
            },
            "required": ["location"]
        }),
    };
    
    let function_prompt = "What's the weather like in New York?";
    println!("Sending function call prompt: \"{}\"", function_prompt);
    
    let function_result = client.call_function(function_prompt, function).await?;
    println!("\nFunction call result:");
    println!("-----------------------------------");
    println!("{}", serde_json::to_string_pretty(&function_result)?);
    
    // Step 6: Demonstrate integration with the toolkit
    println!("\nDemonstrating integration with the toolkit...");
    
    // In a real application, you would register your custom client with the toolkit
    // For this example, we'll just show how it would be used
    
    println!("\nCustom AI client example completed successfully!");
    
    Ok(())
}

/// A mock AI client that simulates responses for demonstration purposes
struct MockAiClient {
    model: String,
    base_url: String,
}

impl MockAiClient {
    /// Create a new mock AI client
    fn new(model: &str, base_url: &str) -> Self {
        Self {
            model: model.to_string(),
            base_url: base_url.to_string(),
        }
    }
    
    /// Generate a mock response based on the prompt
    fn generate_mock_response(&self, prompt: &str, max_tokens: Option<u32>) -> String {
        // In a real implementation, this would call an actual AI API
        // For this example, we'll generate a simple response based on the prompt
        
        let max_tokens = max_tokens.unwrap_or(500);
        
        // Simulate thinking time
        std::thread::sleep(Duration::from_millis(500));
        
        if prompt.contains("artificial intelligence") || prompt.contains("AI") {
            let response = "Artificial Intelligence (AI) refers to computer systems designed to perform tasks \
                           that typically require human intelligence. These tasks include learning, reasoning, \
                           problem-solving, perception, and language understanding. AI can be categorized into \
                           narrow AI (designed for specific tasks) and general AI (capable of performing any \
                           intellectual task that a human can do). Modern AI systems often use machine learning, \
                           particularly deep learning, to improve their performance over time through experience.";
                           
            // Truncate to max_tokens (approximating tokens as characters / 4)
            let char_limit = max_tokens as usize * 4;
            if response.len() > char_limit {
                return response[..char_limit].to_string();
            }
            
            response.to_string()
        } else if prompt.contains("weather") {
            "The weather in New York is currently 72°F and partly cloudy with a 20% chance of rain later today.".to_string()
        } else {
            format!("I received your prompt: \"{}\". This is a mock response from the custom AI client.", prompt)
        }
    }
    
    /// Generate mock streaming chunks from a response
    fn generate_streaming_chunks(&self, response: String) -> Vec<String> {
        // Split the response into chunks to simulate streaming
        let chunk_size = 10; // Characters per chunk
        let mut chunks = Vec::new();
        
        let chars: Vec<char> = response.chars().collect();
        
        for i in (0..chars.len()).step_by(chunk_size) {
            let end = std::cmp::min(i + chunk_size, chars.len());
            let chunk: String = chars[i..end].iter().collect();
            chunks.push(chunk);
        }
        
        chunks
    }
}

#[async_trait]
impl AiClient for MockAiClient {
    fn model_version(&self) -> &str {
        &self.model
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        // Log the request (in a real client, you would send it to the API)
        println!("  [MockAiClient] Received generate request with prompt: \"{}\"", 
                 if prompt.len() > 50 { &prompt[..50] } else { prompt });
        
        // Generate a mock response
        let response = self.generate_mock_response(prompt, None);
        
        Ok(response)
    }
    
    async fn generate_with_options(&self, prompt: &str, options: RequestOptions) -> Result<String> {
        // Log the request with options
        println!("  [MockAiClient] Received generate_with_options request:");
        println!("    Prompt: \"{}\"", if prompt.len() > 50 { &prompt[..50] } else { prompt });
        println!("    Max tokens: {:?}", options.max_tokens);
        println!("    Temperature: {:?}", options.temperature);
        
        // Check if a timeout was specified and honor it
        if let Some(timeout) = options.timeout {
            let start = Instant::now();
            
            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Check if we've exceeded the timeout
            if start.elapsed() > timeout {
                return Err(ToolkitError::Timeout("Request timed out".to_string()));
            }
        }
        
        // Generate a mock response with the specified max_tokens
        let response = self.generate_mock_response(prompt, options.max_tokens);
        
        Ok(response)
    }
    
    async fn generate_streaming(&self, prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        println!("  [MockAiClient] Received generate_streaming request");
        
        // Generate the full response first
        let response = self.generate_mock_response(prompt, None);
        
        // Split it into chunks
        let chunks = self.generate_streaming_chunks(response);
        
        // Create a stream of chunks with delays to simulate streaming
        let stream = stream::iter(chunks)
            .then(|chunk| async move {
                // Add a small delay between chunks to simulate network latency
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(chunk)
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn generate_streaming_with_options(
        &self, 
        prompt: &str, 
        options: RequestOptions
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        println!("  [MockAiClient] Received generate_streaming_with_options request");
        
        // Generate the full response first with the specified max_tokens
        let response = self.generate_mock_response(prompt, options.max_tokens);
        
        // Split it into chunks
        let chunks = self.generate_streaming_chunks(response);
        
        // Create a stream of chunks with delays to simulate streaming
        let stream = stream::iter(chunks)
            .then(|chunk| async move {
                // Add a small delay between chunks to simulate network latency
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(chunk)
            });
        
        Ok(Box::pin(stream))
    }
    
    async fn generate_json(&self, prompt: &str) -> Result<Value> {
        println!("  [MockAiClient] Received generate_json request");
        
        // For a JSON response, we'll create a simple JSON object
        let json = serde_json::json!({
            "response": self.generate_mock_response(prompt, None),
            "metadata": {
                "model": self.model,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "prompt_length": prompt.len()
            }
        });
        
        Ok(json)
    }
    
    async fn call_function(
        &self,
        prompt: &str,
        function: FunctionDefinition,
    ) -> Result<Value> {
        println!("  [MockAiClient] Received call_function request");
        println!("    Function: {}", function.name);
        
        // Simulate function calling by generating a mock response
        // In a real implementation, this would analyze the prompt and call the appropriate function
        
        if function.name == "get_weather" && prompt.contains("weather") {
            // Extract location from prompt (very simplistic for demo purposes)
            let location = if prompt.contains("New York") {
                "New York, NY"
            } else if prompt.contains("San Francisco") {
                "San Francisco, CA"
            } else {
                "Unknown Location"
            };
            
            // Return a mock weather response
            let json = serde_json::json!({
                "function": "get_weather",
                "parameters": {
                    "location": location
                },
                "result": {
                    "temperature": 72,
                    "condition": "partly cloudy",
                    "precipitation_chance": 20,
                    "humidity": 65
                }
            });
            
            Ok(json)
        } else {
            // For unknown functions or unrelated prompts
            Err(ToolkitError::Api(format!(
                "Function '{}' not supported or prompt not relevant", 
                function.name
            )))
        }
    }
} 