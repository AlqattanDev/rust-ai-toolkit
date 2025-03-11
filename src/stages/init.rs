use crate::error::Result;
use crate::models::Project;
use crate::utils::project;
use colored::Colorize;
use crate::config::ColorizeExt;
use nanoid::nanoid;
use std::env;

pub async fn run_init(name: &str, description: &str) -> Result<()> {
    // Get the current directory
    let current_dir = env::current_dir()?;
    
    // Generate a unique ID for the project
    let id = nanoid!(10);
    
    // Create project directory in the current directory
    let project_dir = current_dir.join(name.replace(" ", "-").to_lowercase());
    std::fs::create_dir_all(&project_dir)?;
    
    // Create stages directory
    let stages_dir = project_dir.join("stages");
    std::fs::create_dir_all(&stages_dir)?;
    
    // Create artifacts directory
    let artifacts_dir = project_dir.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)?;
    
    // Create a new project
    let project = Project::new(
        id.clone(),
        name.to_string(),
        description.to_string(),
        project_dir.clone(),
    );
    
    // Save the project
    project::save_project(&project)?;
    
    // Create a file with the initial idea description
    let idea_file = project_dir.join("idea.md");
    std::fs::write(
        &idea_file,
        format!("# {}\n\n{}\n\nCreated at: {}", name, description, project.created_at),
    )?;
    
    println!("{} {} {}", "Project".green(), name.yellow(), "initialized successfully.".green());
    println!("{} {}", "Project ID:".green(), id.yellow());
    println!("{} {}", "Project directory:".green(), project_dir.display().to_string().yellow());
    println!();
    println!("{}", "Use the following commands to manage your project:".green());
    println!("  {} {} - {}", "run-stage".yellow(), "1".bright_blue(), "Run the first stage (Initial Plan Creation)");
    println!("  {} {} {} - {}", "status".yellow(), "-p".bright_blue(), id.bright_blue(), "Check project status");
    
    Ok(())
}
