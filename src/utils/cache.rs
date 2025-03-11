//! Project caching utilities for improved performance.
//!
//! This module provides caching mechanisms to avoid repeated file system operations
//! and improve performance when working with project data. It implements a time-based
//! caching strategy with automatic invalidation of stale entries.
//!
//! The main components are:
//! - [`ProjectCache`]: A cache for project metadata
//! - [`CachedProject`]: A wrapper for cached project data with timestamp information
//! - Global utility functions for working with the shared cache instance
//!
//! # Examples
//!
//! ```no_run
//! use crate::utils::cache;
//! use crate::error::Result;
//!
//! fn example() -> Result<()> {
//!     // Get a project from the cache (or load it if not cached)
//!     let project = cache::get_cached_project("project-123")?;
//!     
//!     // Use the project data
//!     println!("Project name: {}", project.name);
//!     
//!     // Save changes back to disk and update the cache
//!     cache::save_cached_project(&project)?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Thread Safety
//!
//! The global cache instance is protected by a mutex, making it safe to use
//! from multiple threads. However, be mindful of potential contention when
//! accessing the cache frequently from many threads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::models::Project;
use crate::error::Result;
use log::debug;
use crate::config;
use lazy_static::lazy_static;

/// The maximum time a project should be kept in cache before being refreshed.
///
/// After this duration has elapsed, cached projects will be considered stale
/// and will be reloaded from disk on the next access.
const PROJECT_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// The maximum number of projects to keep in the cache
const MAX_CACHE_SIZE: usize = 100;

// Initialize the cache with the TTL from config
lazy_static! {
    /// Global project cache to avoid repeated disk access
    pub static ref PROJECT_CACHE: Mutex<ProjectCache> = {
        let config = config::get_config().unwrap_or_default();
        let ttl = Duration::from_secs(config.project_cache_ttl);
        Mutex::new(ProjectCache::new_with_ttl(ttl))
    };
}

/// Struct for caching project metadata to avoid repeated file operations.
///
/// This struct wraps a [`Project`] instance with timestamp information to track
/// when the data was last refreshed, allowing for time-based cache invalidation.
///
/// # Examples
///
/// ```
/// use crate::models::Project;
/// use crate::utils::cache::CachedProject;
/// use std::path::PathBuf;
///
/// // Create a project
/// let project = Project::new(
///     "project-123".to_string(),
///     "My Project".to_string(),
///     "A test project".to_string(),
///     PathBuf::from("/path/to/project")
/// );
///
/// // Wrap it in a cached project
/// let cached_project = CachedProject::new(project);
///
/// // Check if the cache is still valid
/// if cached_project.is_valid() {
///     println!("Cache is still valid");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CachedProject {
    /// The cached project data.
    pub project: Project,
    /// When this project was last loaded or refreshed.
    pub last_refreshed: Instant,
}

impl CachedProject {
    /// Create a new cached project.
    ///
    /// Initializes a new cached project with the current timestamp.
    ///
    /// # Parameters
    ///
    /// * `project` - The project data to cache.
    ///
    /// # Returns
    ///
    /// A new `CachedProject` instance with the current time as the refresh timestamp.
    pub fn new(project: Project) -> Self {
        Self {
            project,
            last_refreshed: Instant::now(),
        }
    }
    
    /// Check if this cached project is still valid
    pub fn is_valid(&self) -> bool {
        self.last_refreshed.elapsed() < ProjectCache::get_ttl()
    }
}

/// Struct for caching projects to avoid repeated file operations.
///
/// This cache stores project data and directory scan results to minimize
/// file system access and improve performance. It implements time-based
/// invalidation to ensure data freshness.
///
/// # Examples
///
/// ```
/// use crate::utils::cache::ProjectCache;
/// use crate::models::Project;
/// use std::path::PathBuf;
///
/// // Create a new cache
/// let mut cache = ProjectCache::new();
///
/// // Create a project
/// let project = Project::new(
///     "project-123".to_string(),
///     "My Project".to_string(),
///     "A test project".to_string(),
///     PathBuf::from("/path/to/project")
/// );
///
/// // Add the project to the cache
/// cache.insert_project(project);
///
/// // Retrieve the project from the cache
/// if let Some(cached_project) = cache.get_project("project-123") {
///     println!("Found project: {}", cached_project.project.name);
/// }
/// ```
pub struct ProjectCache {
    /// The map of project IDs to their cached data.
    projects: HashMap<String, CachedProject>,
    /// A map of directories to the list of project IDs found there.
    directories: HashMap<PathBuf, Vec<String>>,
    /// Last time a directory was scanned.
    dir_scan_times: HashMap<PathBuf, Instant>,
    /// Queue of project IDs in order of access for LRU eviction
    access_queue: Vec<String>,
    /// Cache TTL
    ttl: Duration,
}

impl ProjectCache {
    /// Create a new empty project cache.
    ///
    /// Initializes an empty cache with no cached projects or directory information.
    ///
    /// # Returns
    ///
    /// A new `ProjectCache` instance.
    pub fn new() -> Self {
        Self {
            projects: HashMap::new(),
            directories: HashMap::new(),
            dir_scan_times: HashMap::new(),
            access_queue: Vec::with_capacity(MAX_CACHE_SIZE),
            ttl: PROJECT_CACHE_TTL,
        }
    }
    
    /// Create a new cache with a specific TTL
    pub fn new_with_ttl(ttl: Duration) -> Self {
        Self {
            projects: HashMap::new(),
            directories: HashMap::new(),
            dir_scan_times: HashMap::new(),
            access_queue: Vec::new(),
            ttl,
        }
    }
    
    /// Get a cached project by ID if it exists and is still valid.
    ///
    /// # Parameters
    ///
    /// * `project_id` - The ID of the project to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the cached project if found and valid,
    /// or `None` if the project is not in the cache or has expired.
    pub fn get_project(&mut self, project_id: &str) -> Option<&CachedProject> {
        if let Some(cached) = self.projects.get(project_id) {
            if cached.is_valid() {
                // Update access order for LRU
                if let Some(pos) = self.access_queue.iter().position(|id| id == project_id) {
                    self.access_queue.remove(pos);
                }
                self.access_queue.push(project_id.to_string());
                
                return Some(cached);
            } else {
                // Will be removed by the caller
                return None;
            }
        }
        None
    }
    
    /// Get a mutable reference to a cached project by ID if it exists and is still valid.
    ///
    /// # Parameters
    ///
    /// * `project_id` - The ID of the project to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the cached project if found and valid,
    /// or `None` if the project is not in the cache or has expired.
    pub fn get_project_mut(&mut self, project_id: &str) -> Option<&mut CachedProject> {
        if let Some(cached) = self.projects.get_mut(project_id) {
            if cached.is_valid() {
                // Update access order for LRU
                if let Some(pos) = self.access_queue.iter().position(|id| id == project_id) {
                    self.access_queue.remove(pos);
                }
                self.access_queue.push(project_id.to_string());
                
                return Some(cached);
            }
        }
        None
    }
    
    /// Insert a project into the cache.
    ///
    /// If the cache is full, the least recently used project will be evicted.
    ///
    /// # Parameters
    ///
    /// * `project` - The project to cache.
    pub fn insert_project(&mut self, project: Project) {
        let project_id = project.id.clone();
        
        // Check if we need to evict entries due to size limit
        if !self.projects.contains_key(&project_id) && self.projects.len() >= MAX_CACHE_SIZE {
            // Evict the least recently used entry
            if let Some(oldest_id) = self.access_queue.first() {
                let oldest_id = oldest_id.clone();
                self.projects.remove(&oldest_id);
                self.access_queue.remove(0);
                debug!("Evicted least recently used project from cache: {}", oldest_id);
            }
        }
        
        // Update access order for LRU
        if let Some(pos) = self.access_queue.iter().position(|id| id == &project_id) {
            self.access_queue.remove(pos);
        }
        self.access_queue.push(project_id);
        
        // Insert the project
        let cached_project = CachedProject::new(project);
        self.projects.insert(cached_project.project.id.clone(), cached_project);
    }
    
    /// Check if a directory scan is still valid.
    ///
    /// Determines if the cached directory scan results are still fresh based on
    /// the time they were last updated and the configured TTL.
    ///
    /// # Parameters
    ///
    /// * `dir` - The directory path to check.
    ///
    /// # Returns
    ///
    /// `true` if the directory scan is still valid, `false` if it has expired or doesn't exist.
    pub fn is_dir_scan_valid(&self, dir: &Path) -> bool {
        if let Some(scan_time) = self.dir_scan_times.get(dir) {
            scan_time.elapsed() < PROJECT_CACHE_TTL
        } else {
            false
        }
    }
    
    /// Record that a directory has been scanned.
    ///
    /// Updates the cache with the results of a directory scan, storing the
    /// list of project IDs found in the directory and the current timestamp.
    ///
    /// # Parameters
    ///
    /// * `dir` - The directory path that was scanned.
    /// * `project_ids` - The list of project IDs found in the directory.
    pub fn record_dir_scan(&mut self, dir: PathBuf, project_ids: Vec<String>) {
        self.dir_scan_times.insert(dir.clone(), Instant::now());
        self.directories.insert(dir, project_ids);
    }
    
    /// Get project IDs found in a directory, if the cache is valid.
    ///
    /// Retrieves the list of project IDs found in a directory if the cached
    /// scan results are still valid.
    ///
    /// # Parameters
    ///
    /// * `dir` - The directory path to check.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the list of project IDs if the cache is valid,
    /// or `None` if the cache is invalid or the directory hasn't been scanned.
    pub fn get_projects_in_dir(&self, dir: &Path) -> Option<&Vec<String>> {
        if self.is_dir_scan_valid(dir) {
            self.directories.get(dir)
        } else {
            None
        }
    }
    
    /// Invalidate the cache for a specific project.
    ///
    /// Removes a project from the cache, forcing it to be reloaded from disk
    /// on the next access.
    ///
    /// # Parameters
    ///
    /// * `project_id` - The ID of the project to invalidate.
    pub fn invalidate_project(&mut self, project_id: &str) {
        self.projects.remove(project_id);
    }
    
    /// Invalidate all directory scans.
    ///
    /// Clears all cached directory scan results, forcing directories to be
    /// rescanned on the next access.
    pub fn invalidate_dir_scans(&mut self) {
        self.directories.clear();
        self.dir_scan_times.clear();
    }
    
    /// Clean the cache by removing expired entries.
    ///
    /// This method removes all expired projects and directory scans from the cache.
    ///
    /// # Returns
    ///
    /// The number of entries that were removed.
    pub fn clean(&mut self) -> usize {
        let mut removed = 0;
        
        // Remove expired projects
        let expired_projects: Vec<String> = self.projects
            .iter()
            .filter(|(_, cached)| !cached.is_valid())
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in &expired_projects {
            self.projects.remove(id);
            if let Some(pos) = self.access_queue.iter().position(|qid| qid == id) {
                self.access_queue.remove(pos);
            }
            removed += 1;
        }
        
        // Remove expired directory scans
        let expired_dirs: Vec<PathBuf> = self.dir_scan_times
            .iter()
            .filter(|(_, time)| time.elapsed() > Duration::from_secs(60))
            .map(|(dir, _)| dir.clone())
            .collect();
        
        for dir in expired_dirs {
            self.dir_scan_times.remove(&dir);
            self.directories.remove(&dir);
            removed += 1;
        }
        
        if removed > 0 {
            debug!("Cleaned {} expired entries from project cache", removed);
        }
        
        removed
    }

    /// Get the size of the cache
    pub fn size(&self) -> usize {
        self.projects.len()
    }

    /// Get the current TTL value
    pub fn get_ttl() -> Duration {
        // Get TTL from config or use default
        match config::get_config() {
            Ok(config) => Duration::from_secs(config.project_cache_ttl),
            Err(_) => PROJECT_CACHE_TTL,
        }
    }
}

/// Get a project from the cache, loading it from disk if necessary or if the cache is stale
pub fn get_cached_project(project_id: &str) -> Result<Project> {
    // Try to get from cache first
    {
        let mut cache = PROJECT_CACHE.lock().unwrap();
        
        // Check if we have a valid cached entry
        if let Some(cached) = cache.get_project_mut(project_id) {
            if cached.is_valid() {
                debug!("Cache hit for project: {}", project_id);
                return Ok(cached.project.clone());
            }
        }
    }
    
    // If we get here, we need to load from disk
    debug!("Cache miss for project: {}, loading from disk", project_id);
    let project = crate::utils::project::load_project_internal(project_id)?;
    
    // Update the cache with the freshly loaded project
    {
        let mut cache = PROJECT_CACHE.lock().unwrap();
        cache.insert_project(project.clone());
        
        // Perform cache cleanup if needed
        if cache.size() > MAX_CACHE_SIZE {
            let removed = cache.clean();
            debug!("Cleaned {} stale entries from project cache", removed);
        }
    }
    
    Ok(project)
}

/// Save a project to disk and update the cache
pub fn save_cached_project(project: &Project) -> Result<()> {
    // Save to disk first
    crate::utils::project::save_project(project)?;
    
    // Then update the cache
    let mut cache = PROJECT_CACHE.lock().unwrap();
    cache.insert_project(project.clone());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;
    use mockall::predicate::*;

    fn create_test_project(id: &str) -> Project {
        Project::new(
            id.to_string(),
            format!("Test Project {}", id),
            "Test Description".to_string(),
            PathBuf::from("/tmp/test")
        )
    }

    #[test]
    fn test_cached_project_validity() {
        let project = create_test_project("test1");
        let cached = CachedProject::new(project);
        
        assert!(cached.is_valid());
        
        // Sleep past TTL
        thread::sleep(PROJECT_CACHE_TTL + Duration::from_secs(1));
        assert!(!cached.is_valid());
    }

    #[test]
    fn test_project_cache_basic_operations() {
        let mut cache = ProjectCache::new();
        let project = create_test_project("test1");
        
        // Test insert and get
        cache.insert_project(project.clone());
        let cached = cache.get_project("test1").unwrap();
        assert_eq!(cached.project.id, "test1");
        
        // Test get_mut
        let cached_mut = cache.get_project_mut("test1").unwrap();
        cached_mut.project.name = "Updated Name".to_string();
        assert_eq!(cache.get_project("test1").unwrap().project.name, "Updated Name");
        
        // Test invalidation
        cache.invalidate_project("test1");
        assert!(cache.get_project("test1").is_none());
    }

    #[test]
    fn test_directory_scanning() {
        let mut cache = ProjectCache::new();
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_path_buf();
        
        // Test directory scan recording
        let project_ids = vec!["test1".to_string(), "test2".to_string()];
        cache.record_dir_scan(dir_path.clone(), project_ids.clone());
        
        // Test directory scan validity
        assert!(cache.is_dir_scan_valid(&dir_path));
        assert_eq!(cache.get_projects_in_dir(&dir_path).unwrap(), &project_ids);
        
        // Test directory scan invalidation
        thread::sleep(PROJECT_CACHE_TTL + Duration::from_secs(1));
        assert!(!cache.is_dir_scan_valid(&dir_path));
        assert!(cache.get_projects_in_dir(&dir_path).is_none());
        
        // Test invalidate_dir_scans
        cache.record_dir_scan(dir_path.clone(), project_ids);
        cache.invalidate_dir_scans();
        assert!(cache.get_projects_in_dir(&dir_path).is_none());
    }

    #[test]
    fn test_concurrent_cache_access() {
        let project1 = create_test_project("test1");
        let project2 = create_test_project("test2");
        
        let mut cache = ProjectCache::new();
        
        // Insert both projects
        cache.insert_project(project1);
        cache.insert_project(project2);
        
        // Verify both projects are in the cache
        assert!(cache.get_project("test1").is_some());
        assert!(cache.get_project("test2").is_some());
    }

    #[test]
    fn test_global_cache_operations() {
        // Test get_cached_project
        let project = create_test_project("test_global");
        save_cached_project(&project).unwrap();
        
        // Should hit cache
        let cached = get_cached_project("test_global").unwrap();
        assert_eq!(cached.id, "test_global");
        
        // Invalidate by waiting
        thread::sleep(PROJECT_CACHE_TTL + Duration::from_secs(1));
        
        // Should miss cache and try to load from disk
        let result = get_cached_project("nonexistent");
        assert!(result.is_err());
    }
} 