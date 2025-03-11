use crate::ai;
use crate::error::{Result, ToolkitError};
use crate::models::StageStatus;
use crate::utils::{project, ui};
use crate::prompts::PromptManager;
use crate::stages::{Stage, StageContext, StageResult};
use async_trait::async_trait;
use log::{debug, error, info};

pub struct Stage4 {
    name: String,
    description: String,
}

impl Stage4 {
    pub fn new() -> Self {
        Self {
            name: "Progress Assessment".to_string(),
            description: "Assess the progress of the project and provide recommendations".to_string(),
        }
    }
}

#[async_trait]
impl Stage for Stage4 {
    fn number(&self) -> u8 {
        4
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn execute(&self, project_id: &str, mut context: StageContext) -> Result<StageResult> {
        info!("Starting Stage 4 for project: {}", project_id);
        
        // Load the project
        let mut project = self.load_project(project_id)?;
        
        // Check if this stage should be skipped
        if self.should_skip(&project)? {
            return Ok(StageResult::skipped("Stage already completed or dependencies not met", context));
        }
        
        ui::print_stage_header(4, &self.name);
        
        // Check if we have the implementation strategy in the context
        let implementation_strategy = if let Some(strategy) = context.get("implementation_strategy") {
            strategy.clone()
        } else {
            // Try to get it from the project
            if let Some(stage3) = project.get_stage(3) {
                stage3.content.clone().unwrap_or_else(|| "No implementation strategy available".to_string())
            } else {
                error!("Stage 3 output not found for project {}", project_id);
                return Err(ToolkitError::InvalidInput(
                    "Stage 3 must be completed before running Stage 4".to_string()
                ));
            }
        };
        
        // Get the current status from the user
        ui::print_info("Please provide a brief summary of the current project status:");
        let current_status = ui::prompt("Current status: ")?;
        
        // Prepare template variables
        let mut template_vars = self.prepare_template_vars(&project, &context);
        template_vars.insert("implementation_strategy".to_string(), implementation_strategy);
        template_vars.insert("current_status".to_string(), current_status);
        
        // Initialize AI client
        debug!("Initializing AI client");
        let ai_client = ai::get_client().await?;
        
        // Create a prompt manager
        let prompt_manager = PromptManager::global()?;
        
        // Render the template
        let variables = PromptManager::vars_to_json(template_vars);
        let prompt = prompt_manager.render(&self.template_name(), &variables)?;
        
        // Send the prompt to the AI
        info!("Sending prompt to AI service");
        let response = match ai_client.generate(&prompt).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("AI service error: {}", e);
                return Err(e);
            }
        };
        
        // Update the project with the AI's response
        info!("Updating project with AI response");
        project.update_stage(4, response.clone(), StageStatus::Completed);
        
        // Save the updated project
        debug!("Saving updated project");
        if let Err(e) = project::save_project(&project) {
            error!("Failed to save project {}: {}", project_id, e);
            return Err(e);
        }
        
        // Update the context with the progress assessment
        context.set("progress_assessment", response);
        
        ui::print_success("Stage 4 completed successfully!");
        
        Ok(StageResult::success(context))
    }
}
