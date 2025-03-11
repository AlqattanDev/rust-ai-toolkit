mod ai;
mod config;
mod error;
mod models;
mod prompts;
mod stages;
mod utils;

use clap::{Parser, Subcommand};
use colored::Colorize;
use error::{Result, ToolkitError, colorize_error};
use log::{debug, error, info};
use dirs;

#[derive(Parser)]
#[command(name = "rust-ai-toolkit")]
#[command(about = "A toolkit for automating staged approach to project planning with AI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project plan
    Init {
        /// Name of the project
        #[arg(short, long)]
        name: String,
        
        /// Brief description of the project idea
        #[arg(short, long)]
        description: String,
    },
    
    /// Run a specific stage of the planning process
    RunStage {
        /// Stage number to run (1-5)
        #[arg(short, long)]
        stage: u8,
        
        /// Project ID to run the stage for
        #[arg(short, long)]
        project: String,
    },
    
    /// List all projects
    List,
    
    /// Show the status of a project
    Status {
        /// Project ID to show status for
        #[arg(short, long)]
        project: String,
    },
    
    /// Configure AI provider settings
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize env_logger with a custom format
    env_logger::builder()
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Seconds))
        .format_module_path(true)
        .init();
    
    info!("Starting Rust AI Toolkit");
    let cli = Cli::parse();
    
    // Create AI client once when needed with caching
    let ai_client = match &cli.command {
        Commands::Init { .. } | Commands::RunStage { .. } | Commands::Status { .. } | Commands::Config => {
            Some(ai::get_cached_client().await?)
        }
        _ => None,
    };
    
    // Initialize prompt manager
    let home_dir = dirs::home_dir().expect("Failed to find home directory");
    let config_dir = home_dir.join(".rust-ai-toolkit");
    let templates_dir = config_dir.join("templates");
    let prompt_manager = match prompts::PromptManager::new(&templates_dir) {
        Ok(pm) => {
            debug!("Prompt manager initialized with template directory: {:?}", templates_dir);
            pm
        },
        Err(e) => {
            error!("Failed to initialize prompt manager: {}", e);
            eprintln!("{}", "Failed to initialize prompt manager. Using default templates.".red());
            // Create an in-memory prompt manager as fallback
            let temp_dir = std::env::temp_dir().join("rust-ai-toolkit-templates");
            prompts::PromptManager::new(&temp_dir).expect("Failed to create temporary prompt manager")
        }
    };
    
    // Initialize all default templates if they don't exist
    for (name, content) in prompts::DEFAULT_TEMPLATES.iter() {
        let template_path = templates_dir.join(format!("{}.hbs", name));
        if !template_path.exists() {
            debug!("Creating default template: {}", name);
            std::fs::create_dir_all(&templates_dir).ok();
            std::fs::write(&template_path, content).ok();
        }
    }
    
    match cli.command {
        Commands::Init { name, description } => {
            info!("Initializing new project: {}", name);
            println!("{}", "Initializing new project...".green());
            match stages::init::run_init(&name, &description).await {
                Ok(_) => {
                    info!("Project initialization successful: {}", name);
                    Ok(())
                },
                Err(e) => {
                    error!("Project initialization failed: {}", e);
                    println!("{}", colorize_error(&e));
                    Err(e)
                }
            }
        }
        Commands::RunStage { stage, project } => {
            info!("Running stage {} for project {}", stage, project);
            
            println!("{} {} {}", "Running stage".green(), stage.to_string().yellow(), "for project".green());
            
            handle_run_stage_command(stage, &project).await
        }
        Commands::List => {
            info!("Listing all projects");
            println!("{}", "Listing all projects...".green());
            handle_list_command().await
        }
        Commands::Status { project } => {
            info!("Showing status for project: {}", project);
            println!("{} {}", "Showing status for project".green(), project.yellow());
            handle_show_command(&project).await
        }
        Commands::Config => {
            info!("Configuring AI provider settings");
            println!("{}", "Configuring AI provider settings...".green());
            match config::configure_ai().await {
                Ok(_) => {
                    info!("Configuration completed successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Configuration failed: {}", e);
                    println!("{}", colorize_error(&e));
                    Err(e)
                }
            }
        }
    }
}

/// Handle the list command to show all projects
async fn handle_list_command() -> Result<()> {
    utils::project::list_projects_async().await
}

/// Handle the show command to display project status
async fn handle_show_command(project_id: &str) -> Result<()> {
    utils::project::show_status(project_id)
}

/// Handle the run stage command
async fn handle_run_stage_command(stage: u8, project_id: &str) -> Result<()> {
    debug!("Running stage {} for project {}", stage, project_id);
    
    // Validate the project ID
    utils::project::validate_project_id(project_id)?;
    
    // Load the project to make sure it exists
    let _project = utils::project::load_project(project_id)?;
    
    // Get the stage implementation
    let stage_impl = stages::get_stage(stage).ok_or_else(|| {
        error!("Invalid stage number: {}", stage);
        ToolkitError::StageNotFound(stage)
    })?;
    
    // Execute the stage
    let context = stages::StageContext::new();
    let result = stage_impl.execute(project_id, context).await?;
    
    if result.is_success() {
        utils::ui::print_success(&format!("Stage {} completed successfully!", stage));
    } else if result.is_skipped() {
        utils::ui::print_warning(&format!("Stage {} was skipped: {}", stage, 
            result.message.unwrap_or_else(|| "No reason provided".to_string())));
    } else {
        utils::ui::print_error(&format!("Stage {} failed: {}", stage,
            result.message.unwrap_or_else(|| "No error message provided".to_string())));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::rate_limiter;
    
    #[test]
    fn test_rate_limiter() {
        // Reset the rate limits for testing
        rate_limiter::set_rate_limit("test_provider", 5);
        
        // Should be able to make requests initially
        assert!(rate_limiter::can_make_request("test_provider"));
        
        // Record some requests
        for _ in 0..5 {
            rate_limiter::record_request("test_provider");
        }
        
        // Should hit the limit
        assert!(!rate_limiter::can_make_request("test_provider"));
        
        // Test failure handling
        let backoff = rate_limiter::record_failure("test_provider");
        assert!(backoff > 0);
        
        // Test success handling
        rate_limiter::record_success("test_provider");
        
        // Still over the limit though
        assert!(!rate_limiter::can_make_request("test_provider"));
    }
}
