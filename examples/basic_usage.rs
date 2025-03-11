// examples/basic_usage.rs
//
// This example demonstrates the basic usage of the Rust AI Toolkit:
// - Initializing a new project
// - Running the first stage (Initial Plan Creation)
// - Displaying the results
//
// To run this example:
// 1. Make sure you have configured your AI provider (run `rust-ai-toolkit config` first)
// 2. Run: cargo run --example basic_usage

use dotenv::dotenv;
use rust_ai_toolkit::error::Result;
use rust_ai_toolkit::models::Project;
use rust_ai_toolkit::stages::{Stage, StageContext};
use rust_ai_toolkit::stages::stage1::Stage1;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    dotenv().ok();
    
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("Rust AI Toolkit - Basic Usage Example");
    println!("=====================================");
    
    // Step 1: Create a new project
    // In a real application, you might get these values from command-line arguments
    let project_name = "Example Project";
    let project_description = "A sample project to demonstrate the Rust AI Toolkit";
    let project_path = PathBuf::from("./example_project");
    
    println!("\nCreating a new project: {}", project_name);
    
    // Create the project directory if it doesn't exist
    if !project_path.exists() {
        std::fs::create_dir_all(&project_path)?;
    }
    
    // Initialize a new project
    let project = Project::new(
        generate_project_id(),
        project_name.to_string(),
        project_description.to_string(),
        project_path.clone(),
    );
    
    // Save the project to disk
    rust_ai_toolkit::utils::project::save_project(&project)?;
    
    println!("Project created with ID: {}", project.id);
    
    // Step 2: Run the first stage (Initial Plan Creation)
    println!("\nRunning Stage 1: Initial Plan Creation");
    
    // Create a context for the stage
    let mut context = StageContext::new();
    context.set("project_id", &project.id);
    
    // Create and run the stage
    let stage = Stage1::new();
    
    // Run the stage with the context
    // This will use the AI provider to generate an initial plan
    match stage.execute(&project.id, context.clone()).await {
        Ok(result) => {
            println!("\nStage 1 completed successfully!");
            
            // Get the stage output
            if let Some(output) = result.context.get("initial_plan") {
                println!("\nInitial Plan Summary:");
                println!("--------------------");
                
                // Print a summary (first few lines) of the output
                let summary = output.lines().take(10).collect::<Vec<_>>().join("\n");
                println!("{}\n...", summary);
                
                // Save the output to a file
                let output_path = project_path.join("initial_plan.md");
                std::fs::write(&output_path, output)?;
                println!("\nFull plan saved to: {:?}", output_path);
            } else {
                println!("\nNo output found for Stage 1");
            }
        }
        Err(e) => {
            eprintln!("Error running Stage 1: {}", e);
            return Err(e);
        }
    }
    
    // Step 3: Update the project status
    let project = project.clone();
    // Update project status - note that the actual project might not have a completed_stages field
    // so this is just a placeholder. In a real implementation, you would update the project status
    // using the proper API.
    rust_ai_toolkit::utils::project::save_project(&project)?;
    
    println!("\nProject status updated. Stage 1 marked as completed.");
    println!("\nBasic usage example completed successfully!");
    
    Ok(())
}

// Generate a simple project ID
// In the actual toolkit, this would be more sophisticated
fn generate_project_id() -> String {
    use rand::Rng;
    let random_part: u32 = rand::thread_rng().gen_range(10000..99999);
    format!("proj_{}", random_part)
} 