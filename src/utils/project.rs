// Remove unused import
use crate::error::{Result, ToolkitError};
use crate::models::Project;
use crate::utils::cache;
use colored::Colorize;
use crate::config::ColorizeExt;
use std::fs;
use std::env;
use std::path::Path;
use log::{debug, error, info, warn};
use tokio::fs as tokio_fs;
use futures::future::{self};
use futures::stream::{StreamExt};
use std::time::{Instant, Duration, SystemTime};
use std::collections::HashMap;
use std::path::PathBuf;

/// Validates a project ID to prevent injection attacks
pub fn validate_project_id(project_id: &str) -> Result<()> {
    // Only allow alphanumeric characters, hyphens, and underscores
    if !project_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        warn!("Invalid project ID format: {}", project_id);
        return Err(ToolkitError::InvalidInput(
            format!("Invalid project ID format. Project IDs must only contain alphanumeric characters, hyphens, and underscores.")
        ));
    }
    
    Ok(())
}

pub fn save_project(project: &Project) -> Result<()> {
    // Serialize the project to JSON
    let json = serde_json::to_string_pretty(project)
        .map_err(|e| ToolkitError::Serialization(e.to_string()))?;
    
    // Create the project directory if it doesn't exist
    fs::create_dir_all(&project.path)
        .map_err(|e| ToolkitError::Io(format!("Failed to create project directory: {}", e)))?;
    
    // Write the project file
    let project_file = project.path.join("project.json");
    debug!("Saving project file to: {}", project_file.display());
    fs::write(project_file, json)?;
    
    // No need to manually update the cache, the cache module handles this
    
    info!("Project saved successfully: {}", project.id);
    Ok(())
}

/// Async version of save_project for async contexts
pub async fn save_project_async(project: &Project) -> Result<()> {
    // Validate project ID
    validate_project_id(&project.id)?;
    
    // Create the project directory if it doesn't exist
    if !project.path.exists() {
        debug!("Creating project directory: {}", project.path.display());
        tokio_fs::create_dir_all(&project.path).await?;
    }
    
    // Convert the project to JSON
    let json = serde_json::to_string_pretty(project).map_err(|e| {
        error!("Failed to serialize project to JSON: {}", e);
        ToolkitError::Serialization(e.to_string())
    })?;
    
    // Save the project file
    let project_file = project.path.join("project.json");
    debug!("Saving project file to: {}", project_file.display());
    tokio_fs::write(project_file, json).await?;
    
    // Update the cache
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        cache.insert_project(project.clone());
    }
    
    info!("Project saved successfully: {}", project.id);
    Ok(())
}

pub fn load_project(project_id: &str) -> Result<Project> {
    // Validate project ID
    validate_project_id(project_id)?;
    
    // Use the utils/cache module for consistent caching
    crate::utils::cache::get_cached_project(project_id)
}

/// Async version of load_project for async contexts
pub async fn load_project_async(project_id: &str) -> Result<Project> {
    // Validate project ID
    validate_project_id(project_id)?;
    
    // Use tokio spawn_blocking to avoid blocking the async runtime with synchronous file I/O
    let project_id = project_id.to_string(); // Clone the string to move into the closure
    tokio::task::spawn_blocking(move || {
        crate::utils::cache::get_cached_project(&project_id)
    }).await.map_err(|e| ToolkitError::Unknown(e.to_string()))?
}

/// Internal function to load a project directly from disk
/// This bypasses the cache and is used by the cache implementation itself
pub(crate) fn load_project_internal(project_id: &str) -> Result<Project> {
    // Validate project ID
    validate_project_id(project_id)?;
    
    debug!("Loading project from disk with ID: {}", project_id);
    
    // First try to find the project in the current directory by ID
    let current_dir = env::current_dir()?;
    debug!("Searching in current directory: {}", current_dir.display());
    
    // Try to find a directory that matches the project_id
    // or contains a project.json file with the matching ID
    let mut project_dir = current_dir.join(project_id);
    let mut found = false;
    
    // Check if project exists directly in current directory
    if project_dir.exists() && project_dir.join("project.json").exists() {
        debug!("Found project directory directly: {}", project_dir.display());
        found = true;
    }
    
    // If not found directly, look in all subdirectories of current directory
    if !found {
        debug!("Project not found directly, searching subdirectories");
        match search_for_project_in_directory(&current_dir, project_id) {
            Ok(Some(path)) => {
                project_dir = path;
                found = true;
                debug!("Found project in subdirectory: {}", project_dir.display());
            },
            Ok(None) => debug!("Project not found in current directory subdirectories"),
            Err(e) => warn!("Error while searching subdirectories: {}", e),
        }
    }
    
    // If still not found, check the configured projects directory
    if !found {
        debug!("Project not found in current directory, checking configured projects directory");
        let config = crate::config::get_config()?;
        let config_projects_dir = &config.projects_dir;
        
        if config_projects_dir.exists() {
            debug!("Checking configured projects directory: {}", config_projects_dir.display());
            
            // First check for direct match in projects directory
            let config_project_dir = config_projects_dir.join(project_id);
            if config_project_dir.exists() && config_project_dir.join("project.json").exists() {
                project_dir = config_project_dir;
                found = true;
                debug!("Found project directly in configured projects directory");
            } else {
                // Search subdirectories of configured projects directory
                match search_for_project_in_directory(config_projects_dir, project_id) {
                    Ok(Some(path)) => {
                        project_dir = path;
                        found = true;
                        debug!("Found project in subdirectory of configured projects directory");
                    },
                    Ok(None) => debug!("Project not found in configured projects directory subdirectories"),
                    Err(e) => warn!("Error while searching configured projects directory: {}", e),
                }
            }
        }
    }
    
    if !found {
        error!("Could not find project with ID: {}", project_id);
        return Err(ToolkitError::ProjectNotFound(project_id.to_string()));
    }
    
    // Read the project file
    let project_file = project_dir.join("project.json");
    debug!("Loading project from file: {}", project_file.display());
    
    let json = fs::read_to_string(&project_file)?;
    let mut project: Project = serde_json::from_str(&json).map_err(|e| {
        error!("Failed to deserialize project file: {}", e);
        ToolkitError::Serialization(e.to_string())
    })?;
    
    // Ensure the path is set correctly
    project.path = project_dir;
    
    info!("Project loaded successfully: {}", project.id);
    Ok(project)
}

fn search_for_project_in_directory(dir: &Path, project_id: &str) -> Result<Option<std::path::PathBuf>> {
    // Check the cache first
    {
        let cache = cache::PROJECT_CACHE.lock().unwrap();
        if let Some(project_ids) = cache.get_projects_in_dir(dir) {
            if project_ids.contains(&project_id.to_string()) {
                // We know the ID exists in this dir, now find the actual path
                let entries = fs::read_dir(dir)?;
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.is_dir() {
                        let potential_project_file = path.join("project.json");
                        
                        if potential_project_file.exists() {
                            debug!("Found potential project file: {}", potential_project_file.display());
                            // Read the project file to check the ID
                            if let Ok(json) = fs::read_to_string(&potential_project_file) {
                                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                                    if project.id == project_id {
                                        debug!("Project ID matches: {}", project_id);
                                        return Ok(Some(path));
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Project ID not in cache for this directory
                debug!("Project ID not found in directory cache: {}", project_id);
                return Ok(None);
            }
        }
    }
    
    debug!("Searching for project {} in directory: {}", project_id, dir.display());
    let entries = fs::read_dir(dir)?;
    
    // Collect all project IDs found in this directory for caching
    let mut found_project_ids = Vec::new();
    let mut result_path = None;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let potential_project_file = path.join("project.json");
            
            if potential_project_file.exists() {
                debug!("Found potential project file: {}", potential_project_file.display());
                // Read the project file to check the ID
                let json = fs::read_to_string(&potential_project_file)?;
                match serde_json::from_str::<Project>(&json) {
                    Ok(project) => {
                        found_project_ids.push(project.id.clone());
                        
                        if project.id == project_id {
                            debug!("Project ID matches: {}", project_id);
                            result_path = Some(path);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse project file {}: {}", potential_project_file.display(), e);
                        continue;
                    }
                };
            }
        }
    }
    
    // Update the cache with all found project IDs
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        cache.record_dir_scan(dir.to_path_buf(), found_project_ids);
    }
    
    Ok(result_path)
}

/// Async version of search_for_project_in_directory
async fn search_for_project_in_directory_async(dir: &Path, project_id: &str) -> Result<Option<std::path::PathBuf>> {
    // Check the cache first
    {
        let cache = cache::PROJECT_CACHE.lock().unwrap();
        if let Some(project_ids) = cache.get_projects_in_dir(dir) {
            if project_ids.contains(&project_id.to_string()) {
                // We know the ID exists in this dir, now find the actual path
                let mut entries = tokio_fs::read_dir(dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    
                    if tokio_fs::metadata(&path).await?.is_dir() {
                        let potential_project_file = path.join("project.json");
                        
                        if tokio_fs::try_exists(&potential_project_file).await? {
                            debug!("Found potential project file: {}", potential_project_file.display());
                            // Read the project file to check the ID
                            if let Ok(json) = tokio_fs::read_to_string(&potential_project_file).await {
                                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                                    if project.id == project_id {
                                        debug!("Project ID matches: {}", project_id);
                                        return Ok(Some(path));
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Project ID not in cache for this directory
                debug!("Project ID not found in directory cache: {}", project_id);
                return Ok(None);
            }
        }
    }
    
    debug!("Searching for project {} in directory: {}", project_id, dir.display());
    let mut entries = tokio_fs::read_dir(dir).await?;
    
    // Collect all project IDs found in this directory for caching
    let mut found_project_ids = Vec::new();
    let mut result_path = None;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        if tokio_fs::metadata(&path).await?.is_dir() {
            let potential_project_file = path.join("project.json");
            
            if tokio_fs::try_exists(&potential_project_file).await? {
                debug!("Found potential project file: {}", potential_project_file.display());
                // Read the project file to check the ID
                match tokio_fs::read_to_string(&potential_project_file).await {
                    Ok(json) => {
                        match serde_json::from_str::<Project>(&json) {
                            Ok(project) => {
                                found_project_ids.push(project.id.clone());
                                
                                if project.id == project_id {
                                    debug!("Project ID matches: {}", project_id);
                                    result_path = Some(path);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse project file {}: {}", potential_project_file.display(), e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read project file {}: {}", potential_project_file.display(), e);
                        continue;
                    }
                };
            }
        }
    }
    
    // Update the cache with all found project IDs
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        cache.record_dir_scan(dir.to_path_buf(), found_project_ids);
    }
    
    Ok(result_path)
}

pub fn list_projects() -> Result<()> {
    let projects = get_all_projects()?;
    
    println!("{:-^50}", " Projects ".green());
    println!("{:<15} | {:<30}", "ID".cyan(), "Name".cyan());
    println!("{:-<50}", "".dimmed());
    
    if projects.is_empty() {
        println!("{}", "No projects found.".yellow());
    } else {
        for project in projects {
            println!("{:<15} | {:<30}", project.id.yellow(), project.name);
        }
    }
    
    println!("{:-<50}", "".dimmed());
    
    Ok(())
}

/// Async version of list_projects
pub async fn list_projects_async() -> Result<()> {
    let projects = get_all_projects_async().await?;
    
    println!("{:-^50}", " Projects ".green());
    println!("{:<15} | {:<30}", "ID".cyan(), "Name".cyan());
    println!("{:-<50}", "".dimmed());
    
    if projects.is_empty() {
        println!("{}", "No projects found.".yellow());
    } else {
        for project in projects {
            println!("{:<15} | {:<30}", project.id.yellow(), project.name);
        }
    }
    
    println!("{:-<50}", "".dimmed());
    
    Ok(())
}

/// Get all projects from both current directory and configured projects directory
pub fn get_all_projects() -> Result<Vec<Project>> {
    let current_dir = env::current_dir()?;
    debug!("Listing projects in current directory: {}", current_dir.display());
    
    let mut projects = Vec::new();
    
    // Look in current directory
    match collect_projects_from_directory(&current_dir) {
        Ok(mut found_projects) => projects.append(&mut found_projects),
        Err(e) => warn!("Error collecting projects from current directory: {}", e),
    }
    
    // Also look in configured projects directory
    let config = crate::config::get_config()?;
    if config.projects_dir.exists() {
        debug!("Listing projects in configured directory: {}", config.projects_dir.display());
        match collect_projects_from_directory(&config.projects_dir) {
            Ok(mut found_projects) => projects.append(&mut found_projects),
            Err(e) => warn!("Error collecting projects from configured directory: {}", e),
        }
    }
    
    Ok(projects)
}

/// Async version of get_all_projects
pub async fn get_all_projects_async() -> Result<Vec<Project>> {
    let current_dir = env::current_dir()?;
    debug!("Listing projects in current directory: {}", current_dir.display());
    
    let mut projects = Vec::new();
    
    // Look in current directory
    match collect_projects_from_directory_async(&current_dir).await {
        Ok(mut found_projects) => projects.append(&mut found_projects),
        Err(e) => warn!("Error collecting projects from current directory: {}", e),
    }
    
    // Also look in configured projects directory
    let config = crate::config::get_config()?;
    if tokio_fs::try_exists(&config.projects_dir).await? {
        debug!("Listing projects in configured directory: {}", config.projects_dir.display());
        match collect_projects_from_directory_async(&config.projects_dir).await {
            Ok(mut found_projects) => projects.append(&mut found_projects),
            Err(e) => warn!("Error collecting projects from configured directory: {}", e),
        }
    }
    
    Ok(projects)
}

// Helper function to collect projects from a directory
fn collect_projects_from_directory(dir: &Path) -> Result<Vec<Project>> {
    // Check the cache first
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        if let Some(project_ids) = cache.get_projects_in_dir(dir) {
            debug!("Using cached project list for directory: {}", dir.display());
            
            // Clone the project IDs to avoid borrowing issues
            let project_ids = project_ids.clone();
            
            let mut projects = Vec::new();
            for project_id in project_ids {
                if let Some(cached_project) = cache.get_project(&project_id) {
                    if cached_project.is_valid() {
                        projects.push(cached_project.project.clone());
                        continue;
                    }
                }
                
                // If we get here, the project wasn't in cache or was invalid
                // Try to load it from disk
                match load_project_internal(&project_id) {
                    Ok(project) => {
                        // Update the cache with the freshly loaded project
                        cache.insert_project(project.clone());
                        projects.push(project);
                    },
                    Err(e) => {
                        // Log the error but continue with other projects
                        error!("Failed to load project {}: {}", project_id, e);
                    }
                }
            }
            
            return Ok(projects);
        }
    }
    
    let entries = fs::read_dir(dir)?;
    let mut projects = Vec::new();
    let mut project_ids = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let project_file = path.join("project.json");
            
            if project_file.exists() {
                debug!("Found project file: {}", project_file.display());
                match fs::read_to_string(&project_file) {
                    Ok(json) => {
                        match serde_json::from_str::<Project>(&json) {
                            Ok(mut project) => {
                                // Ensure the path is set correctly
                                project.path = path;
                                project_ids.push(project.id.clone());
                                projects.push(project);
                            }
                            Err(e) => {
                                warn!("Failed to parse project file {}: {}", project_file.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read project file {}: {}", project_file.display(), e);
                    }
                }
            }
        }
    }
    
    // Update the cache with all found project IDs and projects
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        cache.record_dir_scan(dir.to_path_buf(), project_ids);
        
        // Also cache each project
        for project in &projects {
            cache.insert_project(project.clone());
        }
    }
    
    Ok(projects)
}

/// Async version of collect_projects_from_directory
async fn collect_projects_from_directory_async(dir: &Path) -> Result<Vec<Project>> {
    // Check the cache first
    {
        let cache = cache::PROJECT_CACHE.lock().unwrap();
        if let Some(project_ids) = cache.get_projects_in_dir(dir) {
            debug!("Using cached project list for directory: {}", dir.display());
            
            // Collect futures for loading projects
            let futures: Vec<_> = project_ids.iter().map(|project_id| {
                // For each project ID, check if it's in the cache and valid
                let project_id = project_id.clone();
                async move {
                    // Try to get from cache first
                    {
                        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
                        if let Some(cached_project) = cache.get_project(&project_id) {
                            if cached_project.is_valid() {
                                return Ok(cached_project.project.clone());
                            }
                        }
                    }
                    
                    // If not in cache or invalid, load from disk
                    load_project_async(&project_id).await
                }
            }).collect();
            
            // Execute all futures concurrently
            let results = future::join_all(futures).await;
            
            // Collect successful results
            let projects: Vec<_> = results
                .into_iter()
                .filter_map(|result| match result {
                    Ok(project) => Some(project),
                    Err(e) => {
                        warn!("Failed to load project: {}", e);
                        None
                    }
                })
                .collect();
            
            return Ok(projects);
        }
    }
    
    let mut entries = tokio_fs::read_dir(dir).await?;
    let mut projects: Vec<Project> = Vec::new();
    let mut project_ids = Vec::new();
    let mut project_loading_tasks = Vec::new();
    
    // Collect all project files
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        if tokio_fs::metadata(&path).await?.is_dir() {
            let project_file = path.join("project.json");
            
            if tokio_fs::try_exists(&project_file).await? {
                debug!("Found project file: {}", project_file.display());
                
                // Create a future for loading this project
                let path_clone = path.clone();
                let task = async move {
                    match tokio_fs::read_to_string(&project_file).await {
                        Ok(json) => {
                            match serde_json::from_str::<Project>(&json) {
                                Ok(mut project) => {
                                    // Ensure the path is set correctly
                                    project.path = path_clone;
                                    Some((project.id.clone(), project))
                                }
                                Err(e) => {
                                    warn!("Failed to parse project file {}: {}", project_file.display(), e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read project file {}: {}", project_file.display(), e);
                            None
                        }
                    }
                };
                
                project_loading_tasks.push(task);
            }
        }
    }
    
    // Execute all project loading tasks concurrently
    let results = future::join_all(project_loading_tasks).await;
    
    // Process results
    for result in results {
        if let Some((id, project)) = result {
            project_ids.push(id);
            projects.push(project);
        }
    }
    
    // Update the cache with all found project IDs and projects
    {
        let mut cache = cache::PROJECT_CACHE.lock().unwrap();
        cache.record_dir_scan(dir.to_path_buf(), project_ids);
        
        // Also cache each project
        for project in &projects {
            cache.insert_project(project.clone());
        }
    }
    
    Ok(projects)
}

pub fn show_status(project_id: &str) -> Result<()> {
    debug!("Showing status for project: {}", project_id);
    let project = load_project(project_id)?;
    
    info!("Displaying status for project: {} ({})", project.name, project.id);
    println!("{:-^80}", format!(" Project: {} ", project.name).green());
    println!("ID: {}", project.id.yellow());
    println!("Description: {}", project.description);
    println!("Created: {}", project.created_at);
    println!("Updated: {}", project.updated_at);
    println!("Directory: {}", project.path.display().to_string().yellow());
    println!();
    
    println!("{:-^80}", " Stages ".green());
    
    for stage in &project.stages {
        let status = match stage.status {
            crate::models::StageStatus::NotStarted => "Not Started".red(),
            crate::models::StageStatus::InProgress => "In Progress".yellow(),
            crate::models::StageStatus::Completed => "Completed".green(),
            crate::models::StageStatus::Failed => "Failed".red(),
        };
        
        println!("Stage {}: {} - {}", stage.number, stage.name.cyan(), status);
        println!("  Description: {}", stage.description);
        
        if let Some(completed_at) = &stage.completed_at {
            println!("  Completed: {}", completed_at);
        }
        
        if !stage.artifacts.is_empty() {
            println!("  Artifacts:");
            for artifact in &stage.artifacts {
                println!("    - {} ({})", artifact.name, artifact.path.display());
            }
        }
        
        println!();
    }
    
    Ok(())
}

pub fn get_project_idea(project_id: &str) -> Result<String> {
    debug!("Retrieving project idea for project: {}", project_id);
    
    // Load the project to get its directory
    let project = load_project(project_id)?;
    let project_dir = &project.path;
    
    let idea_file = project_dir.join("idea.md");
    
    if !idea_file.exists() {
        error!("Idea file not found for project {}: {}", project_id, idea_file.display());
        return Err(ToolkitError::File(format!(
            "Idea file not found: {}. Please create an idea.md file in the project directory.",
            idea_file.display()
        )));
    }
    
    debug!("Reading idea file: {}", idea_file.display());
    let content = fs::read_to_string(idea_file)?;
    
    info!("Project idea retrieved successfully for project: {}", project_id);
    Ok(content)
}

/// Cache entry for a project
struct CachedProject {
    /// The cached project
    project: Project,
    /// When this project was cached
    cached_at: Instant,
    /// Last time the project file was modified
    last_modified: SystemTime,
}

impl CachedProject {
    /// Create a new cached project
    fn new(project: Project, last_modified: SystemTime) -> Self {
        Self {
            project,
            cached_at: Instant::now(),
            last_modified,
        }
    }
    
    /// Check if the cache is still valid
    fn is_valid(&self) -> bool {
        // Check if the cache is not too old (5 minutes)
        if self.cached_at.elapsed() > Duration::from_secs(300) {
            return false;
        }
        
        // Check if the file has been modified since we cached it
        if let Ok(metadata) = fs::metadata(self.project.path.join("project.json")) {
            if let Ok(modified) = metadata.modified() {
                return modified <= self.last_modified;
            }
        }
        
        // If we can't check the file, assume it's still valid
        true
    }
}

/// Cache for projects to avoid repeated disk access
struct ProjectCache {
    /// Map of project IDs to cached projects
    projects: HashMap<String, CachedProject>,
    /// Map of directory paths to project IDs in that directory
    dir_projects: HashMap<PathBuf, Vec<String>>,
    /// When directories were last scanned
    dir_scan_times: HashMap<PathBuf, Instant>,
}

impl ProjectCache {
    /// Create a new empty project cache
    fn new() -> Self {
        Self {
            projects: HashMap::new(),
            dir_projects: HashMap::new(),
            dir_scan_times: HashMap::new(),
        }
    }
    
    /// Get a cached project if available and still valid
    fn get_project(&self, project_id: &str) -> Option<&CachedProject> {
        if let Some(cached) = self.projects.get(project_id) {
            if cached.is_valid() {
                return Some(cached);
            }
        }
        None
    }
    
    /// Insert a project into the cache
    fn insert_project(&mut self, project: Project) {
        // Get the last modified time of the project file
        let last_modified = fs::metadata(project.path.join("project.json"))
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());
        
        let project_id = project.id.clone();
        let cached = CachedProject::new(project, last_modified);
        self.projects.insert(project_id, cached);
    }
    
    /// Record the results of a directory scan
    fn record_dir_scan(&mut self, dir: PathBuf, project_ids: Vec<String>) {
        self.dir_projects.insert(dir.clone(), project_ids);
        self.dir_scan_times.insert(dir, Instant::now());
    }
    
    /// Get the project IDs in a directory if the cache is still valid
    fn get_projects_in_dir(&self, dir: &Path) -> Option<&Vec<String>> {
        if let Some(scan_time) = self.dir_scan_times.get(dir) {
            // Directory scan cache is valid for 1 minute
            if scan_time.elapsed() < Duration::from_secs(60) {
                return self.dir_projects.get(dir);
            }
        }
        None
    }
    
    /// Clear expired entries from the cache
    fn clean(&mut self) {
        // Remove expired projects
        self.projects.retain(|_, cached| cached.is_valid());
        
        // Remove expired directory scans
        let expired_dirs: Vec<PathBuf> = self.dir_scan_times
            .iter()
            .filter(|(_, time)| time.elapsed() > Duration::from_secs(60))
            .map(|(dir, _)| dir.clone())
            .collect();
        
        for dir in expired_dirs {
            self.dir_scan_times.remove(&dir);
            self.dir_projects.remove(&dir);
        }
    }
}
