use crate::ai;
use crate::error::Result;
use crate::models::StageStatus;
use crate::utils::{project, ui};
use crate::prompts::PromptManager;
use crate::stages::{Stage, StageContext, StageResult};
use async_trait::async_trait;
use log::{debug, error, info};

pub struct Stage1 {
    name: String,
    description: String,
}

impl Stage1 {
    pub fn new() -> Self {
        Self {
            name: "Initial Plan Creation".to_string(),
            description: "Create an initial plan for the project based on the idea".to_string(),
        }
    }
}

#[async_trait]
impl Stage for Stage1 {
    fn number(&self) -> u8 {
        1
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn dependencies(&self) -> Vec<u8> {
        // Stage 1 has no dependencies
        vec![]
    }
    
    async fn execute(&self, project_id: &str, mut context: StageContext) -> Result<StageResult> {
        info!("Starting Stage 1 for project: {}", project_id);
        
        // Load the project
        let mut project = self.load_project(project_id)?;
        
        // Check if this stage should be skipped
        if self.should_skip(&project)? {
            return Ok(StageResult::skipped("Stage already completed or dependencies not met", context));
        }
        
        ui::print_stage_header(1, &self.name);
        
        // Get the project idea for the prompt
        let project_idea = match project::get_project_idea(project_id) {
            Ok(idea) => idea,
            Err(e) => {
                error!("Failed to get project idea for {}: {}", project_id, e);
                return Err(e);
            }
        };
        
        // Add project idea to the template variables
        let mut template_vars = self.prepare_template_vars(&project, &context);
        template_vars.insert("project_idea".to_string(), project_idea);
        
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
        project.update_stage(1, response.clone(), StageStatus::Completed);
        
        // Save the updated project
        debug!("Saving updated project");
        if let Err(e) = project::save_project(&project) {
            error!("Failed to save project {}: {}", project_id, e);
            return Err(e);
        }
        
        // Update the context with the initial plan
        context.set("initial_plan", response);
        
        ui::print_success("Stage 1 completed successfully!");
        
        Ok(StageResult::success(context))
    }
}
