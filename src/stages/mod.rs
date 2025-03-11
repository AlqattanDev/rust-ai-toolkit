pub mod init;
pub mod stage1;
pub mod stage2;
pub mod stage3;
pub mod stage4;
pub mod stage5;
pub mod stage6;

use crate::error::Result;
use crate::models::{Project, StageStatus};
use crate::utils::{project, ui};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use log::{debug, error, info, warn};
use serde_json::Value;
use anyhow::anyhow;
use crate::error::ToolkitError;

/// The status of a stage execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageExecutionStatus {
    NotStarted,
    Skipped,
    InProgress,
    Completed,
    Failed,
}

impl Display for StageExecutionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Not Started"),
            Self::Skipped => write!(f, "Skipped"),
            Self::InProgress => write!(f, "In Progress"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Context data passed between stages
#[derive(Debug, Clone, Default)]
pub struct StageContext {
    /// Key-value store for passing data between stages
    pub data: HashMap<String, String>,
}

impl StageContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    
    /// Set a value in the context
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }
    
    /// Get a value from the context
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
    
    /// Check if a key exists in the context
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
    
    /// Convert the context to a JSON Value for template rendering
    pub fn to_json(&self) -> Value {
        serde_json::to_value(&self.data).unwrap_or_default()
    }
}

/// Result of a stage execution
#[derive(Debug, Clone)]
pub struct StageResult {
    pub status: StageExecutionStatus,
    pub message: Option<String>,
    pub context: StageContext,
}

impl StageResult {
    /// Create a new successful result
    pub fn success(context: StageContext) -> Self {
        Self {
            status: StageExecutionStatus::Completed,
            message: None,
            context,
        }
    }
    
    /// Create a new failed result
    pub fn failure(message: impl Into<String>, context: StageContext) -> Self {
        Self {
            status: StageExecutionStatus::Failed,
            message: Some(message.into()),
            context,
        }
    }
    
    /// Create a new skipped result
    pub fn skipped(message: impl Into<String>, context: StageContext) -> Self {
        Self {
            status: StageExecutionStatus::Skipped,
            message: Some(message.into()),
            context,
        }
    }
    
    /// Create a new in progress result
    pub fn in_progress(context: StageContext) -> Self {
        Self {
            status: StageExecutionStatus::InProgress,
            message: None,
            context,
        }
    }
    
    /// Check if the result indicates success
    pub fn is_success(&self) -> bool {
        self.status == StageExecutionStatus::Completed
    }
    
    /// Check if the result indicates failure
    pub fn is_failure(&self) -> bool {
        self.status == StageExecutionStatus::Failed
    }
    
    /// Check if the result indicates the stage was skipped
    pub fn is_skipped(&self) -> bool {
        self.status == StageExecutionStatus::Skipped
    }
}

/// A stage in the project development pipeline
#[async_trait]
pub trait Stage: Send + Sync {
    /// Get the number of this stage
    fn number(&self) -> u8;
    
    /// Get the name of this stage
    fn name(&self) -> &str;
    
    /// Get the description of this stage
    fn description(&self) -> &str;
    
    /// Get the dependencies of this stage (other stage numbers that must be completed first)
    fn dependencies(&self) -> Vec<u8> {
        // By default, stages depend on the previous stage
        if self.number() > 1 {
            vec![self.number() - 1]
        } else {
            vec![]
        }
    }
    
    /// Get the template name for this stage
    fn template_name(&self) -> String {
        format!("stage{}", self.number())
    }
    
    /// Prepare the template variables for this stage
    fn prepare_template_vars(&self, project: &Project, context: &StageContext) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        
        // Add project description
        vars.insert("project_description".to_string(), project.description.clone());
        
        // Add all context variables
        for (key, value) in &context.data {
            vars.insert(key.clone(), value.clone());
        }
        
        vars
    }
    
    /// Check if this stage can be executed based on dependencies
    fn can_execute(&self, project: &Project) -> bool {
        let dependencies = self.dependencies();
        
        // If there are no dependencies, we can always execute
        if dependencies.is_empty() {
            return true;
        }
        
        // Check if all dependencies are completed
        for &dep_num in &dependencies {
            if let Some(stage) = project.get_stage(dep_num) {
                if stage.status != StageStatus::Completed {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        true
    }
    
    /// Execute this stage, returning the result
    async fn execute(&self, project_id: &str, context: StageContext) -> Result<StageResult>;
    
    /// Common implementation for loading a project
    fn load_project(&self, project_id: &str) -> Result<Project> {
        debug!("Loading project data for {}", project_id);
        project::load_project(project_id).map_err(|e| {
            error!("Failed to load project {}: {}", project_id, e);
            e
        })
    }
    
    /// Common implementation for checking if a stage should be skipped
    fn should_skip(&self, project: &Project) -> Result<bool> {
        // Check if this stage has already been completed
        if let Some(stage) = project.get_stage(self.number()) {
            if stage.status == StageStatus::Completed {
                warn!("Stage {} has already been completed", self.number());
                ui::print_warning(&format!("Stage {} has already been completed.", self.number()));
                
                if !ui::prompt_yes_no("Do you want to run it again?", false)? {
                    info!("User chose not to run Stage {} again", self.number());
                    return Ok(true);
                }
                
                info!("User chose to run Stage {} again", self.number());
            }
        }
        
        // Check dependencies
        if !self.can_execute(project) {
            let deps = self.dependencies();
            warn!("Dependencies for Stage {} are not met: {:?}", self.number(), deps);
            ui::print_warning(&format!(
                "Dependencies for Stage {} are not met. Please complete stages {:?} first.",
                self.number(), deps
            ));
            return Ok(true);
        }
        
        Ok(false)
    }
}

/// An enum that wraps all possible stage implementations
/// This allows us to avoid using dyn trait objects with async functions
pub enum StageEnum {
    Stage1(stage1::Stage1),
    Stage2(stage2::Stage2),
    Stage3(stage3::Stage3),
    Stage4(stage4::Stage4),
    Stage5(stage5::Stage5),
    Stage6(stage6::Stage6),
}

impl StageEnum {
    /// Get the number of this stage
    pub fn number(&self) -> u8 {
        match self {
            StageEnum::Stage1(s) => s.number(),
            StageEnum::Stage2(s) => s.number(),
            StageEnum::Stage3(s) => s.number(),
            StageEnum::Stage4(s) => s.number(),
            StageEnum::Stage5(s) => s.number(),
            StageEnum::Stage6(s) => s.number(),
        }
    }
    
    /// Get the name of this stage
    pub fn name(&self) -> &str {
        match self {
            StageEnum::Stage1(s) => s.name(),
            StageEnum::Stage2(s) => s.name(),
            StageEnum::Stage3(s) => s.name(),
            StageEnum::Stage4(s) => s.name(),
            StageEnum::Stage5(s) => s.name(),
            StageEnum::Stage6(s) => s.name(),
        }
    }
    
    /// Get the dependencies of this stage
    pub fn dependencies(&self) -> Vec<u8> {
        match self {
            StageEnum::Stage1(s) => s.dependencies(),
            StageEnum::Stage2(s) => s.dependencies(),
            StageEnum::Stage3(s) => s.dependencies(),
            StageEnum::Stage4(s) => s.dependencies(),
            StageEnum::Stage5(s) => s.dependencies(),
            StageEnum::Stage6(s) => s.dependencies(),
        }
    }
    
    /// Execute this stage
    pub async fn execute(&self, project_id: &str, context: StageContext) -> Result<StageResult> {
        match self {
            StageEnum::Stage1(s) => s.execute(project_id, context).await,
            StageEnum::Stage2(s) => s.execute(project_id, context).await,
            StageEnum::Stage3(s) => s.execute(project_id, context).await,
            StageEnum::Stage4(s) => s.execute(project_id, context).await,
            StageEnum::Stage5(s) => s.execute(project_id, context).await,
            StageEnum::Stage6(s) => s.execute(project_id, context).await,
        }
    }
}

/// Get a stage by its number
pub fn get_stage(stage_number: u8) -> Option<StageEnum> {
    match stage_number {
        1 => Some(StageEnum::Stage1(stage1::Stage1::new())),
        2 => Some(StageEnum::Stage2(stage2::Stage2::new())),
        3 => Some(StageEnum::Stage3(stage3::Stage3::new())),
        4 => Some(StageEnum::Stage4(stage4::Stage4::new())),
        5 => Some(StageEnum::Stage5(stage5::Stage5::new())),
        6 => Some(StageEnum::Stage6(stage6::Stage6::new())),
        _ => None,
    }
}

/// Run a sequence of stages for a project
pub async fn run_stages(project_id: &str, stages: &[u8]) -> Result<StageContext> {
    let mut context = StageContext::new();
    
    for &stage_number in stages {
        if let Some(stage) = get_stage(stage_number) {
            println!("Running stage {}: {}", stage_number, stage.name());
            let result = stage.execute(project_id, context.clone()).await?;
            
            if result.is_failure() {
                error!("Stage {} failed: {:?}", stage_number, result.message);
                if let Some(msg) = &result.message {
                    ui::print_error(&format!("Stage {} failed: {}", stage_number, msg));
                } else {
                    ui::print_error(&format!("Stage {} failed", stage_number));
                }
                return Err(ToolkitError::Unknown(format!("Stage {} failed", stage_number)));
            }
            
            if result.is_skipped() {
                info!("Stage {} was skipped", stage_number);
                if let Some(msg) = &result.message {
                    ui::print_info(&format!("Stage {} skipped: {}", stage_number, msg));
                } else {
                    ui::print_info(&format!("Stage {} was skipped", stage_number));
                }
                continue;
            }
            
            // Update context for the next stage
            context = result.context;
            
            // Mark stage as completed in project
            ui::print_success(&format!("Stage {} completed successfully", stage_number));
            info!("Stage {} completed successfully", stage_number);
        } else {
            ui::print_error(&format!("Invalid stage number: {}", stage_number));
            return Err(ToolkitError::StageNotFound(stage_number));
        }
    }
    
    Ok(context)
}

/// Run all stages for a project in sequence
pub async fn run_all_stages(project_id: &str) -> Result<StageContext> {
    run_stages(project_id, &[1, 2, 3, 4, 5, 6]).await
}

/// Run all available stages for a project based on dependencies
pub async fn run_available_stages(project_id: &str) -> Result<StageContext> {
    let mut context = StageContext::new();
    
    for stage_num in 1..=6 {
        if let Some(stage) = get_stage(stage_num) {
            // Check dependencies
            let deps = stage.dependencies();
            
            // Skip if dependencies aren't met
            let mut can_run = true;
            for &dep in &deps {
                let project = project::load_project(project_id)?;
                if let Some(dep_stage) = project.get_stage(dep) {
                    if dep_stage.status != StageStatus::Completed {
                        can_run = false;
                        break;
                    }
                } else {
                    can_run = false;
                    break;
                }
            }
            
            if can_run {
                println!("Running stage {}: {}", stage_num, stage.name());
                let result = stage.execute(project_id, context.clone()).await?;
                
                if result.is_failure() {
                    error!("Stage {} failed: {:?}", stage_num, result.message);
                    if let Some(msg) = &result.message {
                        ui::print_error(&format!("Stage {} failed: {}", stage_num, msg));
                    } else {
                        ui::print_error(&format!("Stage {} failed", stage_num));
                    }
                    return Err(ToolkitError::Unknown(format!("Stage {} failed", stage_num)));
                }
                
                if result.is_skipped() {
                    info!("Stage {} was skipped", stage_num);
                    if let Some(msg) = &result.message {
                        ui::print_info(&format!("Stage {} skipped: {}", stage_num, msg));
                    } else {
                        ui::print_info(&format!("Stage {} was skipped", stage_num));
                    }
                    continue;
                }
                
                // Update context for the next stage
                context = result.context;
                
                // Mark stage as completed in project
                ui::print_success(&format!("Stage {} completed successfully", stage_num));
                info!("Stage {} completed successfully", stage_num);
            }
        }
    }
    
    Ok(context)
}
