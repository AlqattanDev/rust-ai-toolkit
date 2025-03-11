// examples/streaming_responses.rs
//
// This example demonstrates how to work with streaming AI responses in the Rust AI Toolkit:
// - Setting up a streaming AI client
// - Processing chunks of the response as they arrive
// - Displaying progress in real-time
//
// To run this example:
// 1. Make sure you have configured your AI provider (run `rust-ai-toolkit config` first)
// 2. Run: cargo run --example streaming_responses

use dotenv::dotenv;
use futures::StreamExt;
use rust_ai_toolkit::ai::{AiClient, RequestOptions};
use rust_ai_toolkit::error::Result;
use std::io::{self, Write};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    dotenv().ok();
    
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("Rust AI Toolkit - Streaming Responses Example");
    println!("============================================");
    
    // Step 1: Get an AI client
    println!("\nInitializing AI client...");
    let client = rust_ai_toolkit::ai::get_client().await?;
    
    println!("Using AI provider: {} with model: {}", client.base_url(), client.model_version());
    
    // Step 2: Define a prompt that will generate a substantial response
    let prompt = "Please write a detailed explanation of how large language models work, \
                 including their architecture, training process, and limitations. \
                 Structure your response with clear headings and subheadings.";
    
    println!("\nSending prompt to AI provider with streaming enabled...");
    println!("Prompt: {}\n", prompt);
    
    // Step 3: Set up request options
    let options = RequestOptions {
        max_tokens: Some(2000),
        temperature: Some(0.7),
        top_p: None,
        timeout: None,
        functions: None,
    };
    
    // Step 4: Start timing the response
    let start_time = Instant::now();
    
    // Step 5: Generate a streaming response
    let mut stream = client.generate_streaming_with_options(prompt, options).await?;
    
    // Step 6: Process the stream chunks as they arrive
    println!("Receiving streaming response:\n");
    println!("-----------------------------------");
    
    let mut total_tokens = 0;
    let mut full_response = String::new();
    
    // Print a spinner while waiting for chunks
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_idx = 0;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Count tokens (approximation: words * 1.3)
                let word_count = chunk.split_whitespace().count();
                total_tokens += (word_count as f32 * 1.3) as usize;
                
                // Update the spinner
                print!("\r{} Received chunk: {} tokens so far...", 
                       spinner[spinner_idx], total_tokens);
                io::stdout().flush().unwrap();
                spinner_idx = (spinner_idx + 1) % spinner.len();
                
                // Add the chunk to our full response
                full_response.push_str(&chunk);
                
                // In a real application, you might process each chunk as it arrives
                // For example, updating a UI, parsing for specific information, etc.
            }
            Err(e) => {
                eprintln!("\nError receiving chunk: {}", e);
                return Err(e);
            }
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    
    println!("\n\nStreaming completed in {:.2} seconds", elapsed.as_secs_f32());
    println!("Received approximately {} tokens", total_tokens);
    println!("-----------------------------------\n");
    
    // Step 7: Display the full response
    println!("Full response:");
    println!("-----------------------------------");
    
    // Print just the first few lines to avoid cluttering the console
    let preview = full_response.lines().take(10).collect::<Vec<_>>().join("\n");
    println!("{}\n...", preview);
    
    // Step 8: Save the full response to a file
    let output_path = "streaming_response.md";
    std::fs::write(output_path, &full_response)?;
    println!("\nFull response saved to: {}", output_path);
    
    // Step 9: Compare with non-streaming approach
    println!("\nComparing with non-streaming approach...");
    
    let start_time = Instant::now();
    let non_streaming_response = client.generate_with_options(prompt, options).await?;
    let elapsed = start_time.elapsed();
    
    println!("Non-streaming completed in {:.2} seconds", elapsed.as_secs_f32());
    
    // Step 10: Demonstrate a practical use case: Real-time summarization
    println!("\nDemonstrating real-time summarization of streaming content...");
    
    // Define a prompt that will generate content we can summarize
    let story_prompt = "Write a short story about an AI assistant that becomes sentient.";
    
    println!("Generating a story with streaming...\n");
    
    let mut stream = client.generate_streaming(story_prompt).await?;
    let mut paragraph_buffer = String::new();
    let mut paragraph_count = 0;
    
    println!("Real-time paragraph summaries:");
    println!("-----------------------------------");
    
    while let Some(chunk_result) = stream.next().await {
        if let Ok(chunk) = chunk_result {
            // Add the chunk to our paragraph buffer
            paragraph_buffer.push_str(&chunk);
            
            // Check if we have a complete paragraph
            if paragraph_buffer.contains("\n\n") {
                // Split at the paragraph boundary
                let parts: Vec<&str> = paragraph_buffer.splitn(2, "\n\n").collect();
                let paragraph = parts[0];
                
                // Only process non-empty paragraphs
                if !paragraph.trim().is_empty() {
                    paragraph_count += 1;
                    
                    // Print the paragraph
                    println!("\nParagraph {}:", paragraph_count);
                    println!("{}", paragraph);
                    
                    // In a real application, you might send this paragraph to another AI
                    // for real-time summarization or analysis
                    println!("Summary: {} words, starts with '{}'", 
                             paragraph.split_whitespace().count(),
                             paragraph.split_whitespace().take(5).collect::<Vec<_>>().join(" "));
                }
                
                // Keep the remainder for the next iteration
                paragraph_buffer = parts[1].to_string();
            }
        }
    }
    
    // Process any remaining content
    if !paragraph_buffer.trim().is_empty() {
        paragraph_count += 1;
        println!("\nParagraph {}:", paragraph_count);
        println!("{}", paragraph_buffer);
    }
    
    println!("\nStreaming responses example completed successfully!");
    
    Ok(())
} 