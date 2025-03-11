use crate::ai;
use crate::error::{Result, ToolkitError};
use crate::models::StageStatus;
use crate::utils::{project, ui};
use crate::prompts::PromptManager;
use crate::stages::{Stage, StageContext, StageResult};
use async_trait::async_trait;
use log::{debug, error, info};

pub struct Stage2 {
    name: String,
    description: String,
}

impl Stage2 {
    pub fn new() -> Self {
        Self {
            name: "Architecture Design".to_string(),
            description: "Design the architecture for the project".to_string(),
        }
    }
}

#[async_trait]
impl Stage for Stage2 {
    fn number(&self) -> u8 {
        2
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn dependencies(&self) -> Vec<u8> {
        vec![1] // Depends on stage 1
    }
    
    async fn execute(&self, project_id: &str, mut context: StageContext) -> Result<StageResult> {
        info!("Starting Stage 2 for project: {}", project_id);
        
        // Load the project
        let mut project = self.load_project(project_id)?;
        
        // Check if this stage should be skipped
        if self.should_skip(&project)? {
            return Ok(StageResult::skipped("Stage already completed or dependencies not met", context));
        }
        
        ui::print_stage_header(2, &self.name);
        
        // Check if we have the initial plan in the context
        let initial_plan = if let Some(plan) = context.get("initial_plan") {
            plan.clone()
        } else {
            // Try to get it from the project
            if let Some(stage1) = project.get_stage(1) {
                stage1.content.clone().unwrap_or_else(|| "No initial plan available".to_string())
            } else {
                error!("Stage 1 output not found for project {}", project_id);
                return Err(ToolkitError::InvalidInput(
                    "Stage 1 must be completed before running Stage 2".to_string()
                ));
            }
        };
        
        // Prepare template variables
        let mut template_vars = self.prepare_template_vars(&project, &context);
        template_vars.insert("initial_plan".to_string(), initial_plan);
        
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
        project.update_stage(2, response.clone(), StageStatus::Completed);
        
        // Save the updated project
        debug!("Saving updated project");
        if let Err(e) = project::save_project(&project) {
            error!("Failed to save project {}: {}", project_id, e);
            return Err(e);
        }
        
        // Update the context with the architecture design
        context.set("architecture_design", response);
        
        ui::print_success("Stage 2 completed successfully!");
        
        Ok(StageResult::success(context))
    }
}
