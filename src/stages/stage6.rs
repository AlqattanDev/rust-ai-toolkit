use crate::ai;
use crate::error::Result;
use crate::models::StageStatus;
use crate::utils::{project, ui};
use crate::prompts::PromptManager;
use crate::stages::{Stage, StageContext, StageResult};
use async_trait::async_trait;
use log::{debug, error, info};

pub struct Stage6 {
    name: String,
    description: String,
}

impl Stage6 {
    pub fn new() -> Self {
        Self {
            name: "Code Generation and Review".to_string(),
            description: "Generate and review code for key components of the project".to_string(),
        }
    }
}

#[async_trait]
impl Stage for Stage6 {
    fn number(&self) -> u8 {
        6
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn execute(&self, project_id: &str, mut context: StageContext) -> Result<StageResult> {
        info!("Starting Stage 6 for project: {}", project_id);
        
        // Load the project
        let mut project = self.load_project(project_id)?;
        
        // Check if this stage should be skipped
        if self.should_skip(&project)? {
            return Ok(StageResult::skipped("Stage already completed or dependencies not met", context));
        }
        
        ui::print_stage_header(6, &self.name);
        
        // Gather required context from previous stages if not already in context
        let mut template_vars = self.prepare_template_vars(&project, &context);
        
        // Architecture design
        if !context.has("architecture_design") {
            if let Some(stage2) = project.get_stage(2) {
                template_vars.insert("architecture_design".to_string(), stage2.content.clone().unwrap_or_else(|| "No architecture design available".to_string()));
            }
        }
        
        // Implementation strategy
        if !context.has("implementation_strategy") {
            if let Some(stage3) = project.get_stage(3) {
                template_vars.insert("implementation_strategy".to_string(), stage3.content.clone().unwrap_or_else(|| "No implementation strategy available".to_string()));
            }
        }
        
        // UX design
        if !context.has("ux_design") {
            if let Some(stage5) = project.get_stage(5) {
                template_vars.insert("ux_design".to_string(), stage5.content.clone().unwrap_or_else(|| "No UX design available".to_string()));
            }
        }
        
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
        project.update_stage(6, response.clone(), StageStatus::Completed);
        
        // Save the updated project
        debug!("Saving updated project");
        if let Err(e) = project::save_project(&project) {
            error!("Failed to save project {}: {}", project_id, e);
            return Err(e);
        }
        
        // Update the context with the code generation and review
        context.set("code_generation", response);
        
        ui::print_success("Stage 6 completed successfully!");
        
        Ok(StageResult::success(context))
    }
}
