// examples/custom_template.rs
//
// This example demonstrates how to create and use custom prompt templates with the Rust AI Toolkit:
// - Creating a custom template
// - Registering the template with the PromptManager
// - Using the custom template for a stage
//
// To run this example:
// 1. Make sure you have configured your AI provider (run `rust-ai-toolkit config` first)
// 2. Run: cargo run --example custom_template

use dotenv::dotenv;
use rust_ai_toolkit::error::Result;
use rust_ai_toolkit::prompts::PromptManager;
use rust_ai_toolkit::models::Project;
use rust_ai_toolkit::stages::{Stage, Stage1InitialPlan, Context};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    dotenv().ok();
    
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("Rust AI Toolkit - Custom Template Example");
    println!("========================================");
    
    // Step 1: Create a custom template
    println!("\nCreating a custom template for Stage 1");
    
    // Define our custom template for the initial plan stage
    let custom_template = r#"# Custom Initial Plan Creation for {{project_name}}

## Project Overview
{{project_description}}

## Task
Please analyze this project idea and create a comprehensive plan that includes:

1. **Core Features**
   - What are the essential features this project needs?
   - What would make this project stand out?

2. **Technical Architecture**
   - What components will be needed?
   - How will they interact?
   - What technologies would be most appropriate?

3. **Implementation Roadmap**
   - What are the key milestones?
   - What is a realistic timeline?
   - What are the dependencies between components?

4. **Potential Challenges**
   - What technical challenges might arise?
   - What are potential solutions to these challenges?

5. **Resource Requirements**
   - What skills would be needed?
   - What tools or services would be required?

Please be specific and detailed in your analysis. Format your response in Markdown with clear sections and structure.
"#;
    
    // Step 2: Initialize the PromptManager and register our custom template
    println!("Initializing PromptManager and registering custom template");
    
    // Create a temporary directory for templates
    let temp_dir = tempfile::tempdir()?;
    let template_dir = temp_dir.path();
    
    // Initialize the PromptManager with our temporary directory
    let mut prompt_manager = PromptManager::new(template_dir)?;
    
    // Register our custom template
    prompt_manager.add_template("custom_stage1", custom_template)?;
    
    println!("Custom template registered as 'custom_stage1'");
    
    // Step 3: Create a project to use with our custom template
    let project = Project::new(
        format!("proj_{}", rand::random::<u32>() % 100000),
        "Custom Template Project".to_string(),
        "A project to demonstrate custom templates in the Rust AI Toolkit".to_string(),
        PathBuf::from("./custom_template_project"),
    );
    
    println!("\nCreated project: {}", project.name);
    
    // Step 4: Create a custom Stage1 implementation that uses our template
    println!("Creating a custom Stage1 implementation");
    
    struct CustomStage1 {
        prompt_manager: PromptManager,
    }
    
    impl CustomStage1 {
        fn new(prompt_manager: PromptManager) -> Self {
            Self { prompt_manager }
        }
    }
    
    #[async_trait::async_trait]
    impl Stage for CustomStage1 {
        fn name(&self) -> &str {
            "Custom Initial Plan"
        }
        
        fn stage_number(&self) -> u32 {
            1
        }
        
        fn description(&self) -> &str {
            "Creates an initial project plan using a custom template"
        }
        
        fn dependencies(&self) -> Vec<u32> {
            // No dependencies for Stage 1
            vec![]
        }
        
        async fn run(&self, context: &mut Context) -> Result<()> {
            println!("Running custom Stage 1 with custom template");
            
            // Get the project from the context
            let project = context.project()?;
            
            // Prepare variables for the template
            let mut vars = HashMap::new();
            vars.insert("project_name".to_string(), project.name.clone());
            vars.insert("project_description".to_string(), project.description.clone());
            
            // Convert variables to JSON for the template
            let data = PromptManager::vars_to_json(vars);
            
            // Render the template
            let prompt = self.prompt_manager.render("custom_stage1", &data)?;
            
            println!("\nGenerated prompt from custom template:");
            println!("--------------------------------------");
            println!("{}", prompt);
            
            // Get an AI client
            let ai_client = rust_ai_toolkit::ai::get_client().await?;
            
            println!("\nSending prompt to AI provider...");
            
            // Generate a response from the AI
            let response = ai_client.generate(&prompt).await?;
            
            println!("Received response from AI provider");
            
            // Store the response in the context
            context.set_output("initial_plan", response);
            
            // Save the output to a file
            if !project.path.exists() {
                std::fs::create_dir_all(&project.path)?;
            }
            
            let output_path = project.path.join("custom_initial_plan.md");
            if let Some(output) = context.get_output("initial_plan") {
                std::fs::write(&output_path, output)?;
                println!("\nSaved plan to: {:?}", output_path);
            }
            
            Ok(())
        }
    }
    
    // Step 5: Run our custom stage
    println!("\nRunning the custom stage");
    
    // Create a context for the stage
    let mut context = Context::new();
    context.set_project(project);
    
    // Create and run our custom stage
    let custom_stage = CustomStage1::new(prompt_manager);
    
    match custom_stage.run(&mut context).await {
        Ok(_) => {
            println!("\nCustom stage completed successfully!");
            
            // Get the stage output
            if let Some(output) = context.get_output("initial_plan") {
                println!("\nInitial Plan Summary (first 5 lines):");
                println!("-------------------------------------");
                
                // Print a summary (first few lines) of the output
                let summary = output.lines().take(5).collect::<Vec<_>>().join("\n");
                println!("{}\n...", summary);
            } else {
                println!("\nNo output found for custom stage");
            }
        }
        Err(e) => {
            eprintln!("Error running custom stage: {}", e);
            return Err(e);
        }
    }
    
    println!("\nCustom template example completed successfully!");
    
    Ok(())
} 