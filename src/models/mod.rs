use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub stages: Vec<Stage>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub number: u8,
    pub name: String,
    pub description: String,
    pub status: StageStatus,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub content: Option<String>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StageStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub file_type: String,
    pub path: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Project {
    pub fn new(id: String, name: String, description: String, path: PathBuf) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id,
            name,
            description,
            created_at: now,
            updated_at: now,
            stages: vec![
                Stage {
                    number: 1,
                    name: "Initial Plan Creation".to_string(),
                    description: "Develop a comprehensive plan based on the initial idea".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
                Stage {
                    number: 2,
                    name: "Critical Evaluation".to_string(),
                    description: "Analyze and identify overly complex or impractical elements".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
                Stage {
                    number: 3,
                    name: "Realistic Alternative".to_string(),
                    description: "Propose a more practical, achievable alternative approach".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
                Stage {
                    number: 4,
                    name: "Technical Approach Refinement".to_string(),
                    description: "Compare different technical implementation options".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
                Stage {
                    number: 5,
                    name: "AI Implementation Enhancement".to_string(),
                    description: "Restructure the plan for AI-assisted development".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
                Stage {
                    number: 6,
                    name: "Code Review and Optimization".to_string(),
                    description: "Review and optimize code using Claude Code".to_string(),
                    status: StageStatus::NotStarted,
                    completed_at: None,
                    content: None,
                    artifacts: vec![],
                },
            ],
            path,
        }
    }
    
    pub fn get_stage(&self, stage_number: u8) -> Option<&Stage> {
        self.stages.iter().find(|s| s.number == stage_number)
    }
    
    pub fn get_stage_mut(&mut self, stage_number: u8) -> Option<&mut Stage> {
        self.stages.iter_mut().find(|s| s.number == stage_number)
    }
    
    pub fn update_stage(&mut self, stage_number: u8, content: String, status: StageStatus) -> bool {
        if let Some(stage) = self.get_stage_mut(stage_number) {
            stage.content = Some(content);
            
            // Check if it will be completed before setting the status
            let is_completed = status == StageStatus::Completed;
            
            // Set the status
            stage.status = status;
            
            // Update completed_at timestamp if needed
            if is_completed {
                stage.completed_at = Some(chrono::Utc::now());
            }
            
            self.updated_at = chrono::Utc::now();
            return true;
        }
        
        false
    }
    
    pub fn add_artifact(&mut self, stage_number: u8, artifact: Artifact) -> bool {
        if let Some(stage) = self.get_stage_mut(stage_number) {
            stage.artifacts.push(artifact);
            self.updated_at = chrono::Utc::now();
            return true;
        }
        
        false
    }
}
