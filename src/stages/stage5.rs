use crate::ai;
use crate::error::{Result, ToolkitError};
use crate::models::StageStatus;
use crate::utils::{project, ui};
use crate::prompts::PromptManager;
use crate::stages::{Stage, StageContext, StageResult};
use async_trait::async_trait;
use log::{debug, error, info};

pub struct Stage5 {
    name: String,
    description: String,
}

impl Stage5 {
    pub fn new() -> Self {
        Self {
            name: "User Experience Design".to_string(),
            description: "Design the user experience for the project".to_string(),
        }
    }
}

#[async_trait]
impl Stage for Stage5 {
    fn number(&self) -> u8 {
        5
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn execute(&self, project_id: &str, mut context: StageContext) -> Result<StageResult> {
        info!("Starting Stage 5 for project: {}", project_id);
        
        // Load the project
        let mut project = self.load_project(project_id)?;
        
        // Check if this stage should be skipped
        if self.should_skip(&project)? {
            return Ok(StageResult::skipped("Stage already completed or dependencies not met", context));
        }
        
        ui::print_stage_header(5, &self.name);
        
        // Check if we have the architecture design in the context
        let architecture_design = if let Some(design) = context.get("architecture_design") {
            design.clone()
        } else {
            // Try to get it from the project
            if let Some(stage2) = project.get_stage(2) {
                stage2.content.clone().unwrap_or_else(|| "No architecture design available".to_string())
            } else {
                error!("Stage 2 output not found for project {}", project_id);
                return Err(ToolkitError::InvalidInput(
                    "Stage 2 must be completed before running Stage 5".to_string()
                ));
            }
        };
        
        // Prepare template variables
        let mut template_vars = self.prepare_template_vars(&project, &context);
        template_vars.insert("architecture_design".to_string(), architecture_design);
        
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
        project.update_stage(5, response.clone(), StageStatus::Completed);
        
        // Save the updated project
        debug!("Saving updated project");
        if let Err(e) = project::save_project(&project) {
            error!("Failed to save project {}: {}", project_id, e);
            return Err(e);
        }
        
        // Update the context with the UX design
        context.set("ux_design", response);
        
        ui::print_success("Stage 5 completed successfully!");
        
        Ok(StageResult::success(context))
    }
}
