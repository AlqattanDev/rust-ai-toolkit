use std::fs;
use std::io::{Write};
use std::path::{Path, PathBuf};
use log::{debug, error};
use crate::error::{Result, ToolkitError};
use tokio::fs as tokio_fs;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        debug!("Creating directory: {:?}", path);
        fs::create_dir_all(path).map_err(|e| {
            error!("Failed to create directory {:?}: {}", path, e);
            ToolkitError::Io(e.to_string())
        })?;
    }
    
    Ok(())
}

/// Read the contents of a file as a string
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    debug!("Reading file: {:?}", path);
    
    fs::read_to_string(path).map_err(|e| {
        error!("Failed to read file {:?}: {}", path, e);
        ToolkitError::Io(e.to_string())
    })
}

/// Write a string to a file, creating parent directories if needed
pub fn write_string_to_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Writing to file: {:?}", path);
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)?;
    }
    
    fs::write(path, content).map_err(|e| {
        error!("Failed to write to file {:?}: {}", path, e);
        ToolkitError::Io(e.to_string())
    })
}

/// Append a string to a file, creating parent directories if needed
pub fn append_string_to_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Appending to file: {:?}", path);
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)?;
    }
    
    let mut file = if path.exists() {
        fs::OpenOptions::new()
            .append(true)
            .open(path)
    } else {
        fs::File::create(path)
    }.map_err(|e| {
        error!("Failed to open file {:?} for appending: {}", path, e);
        ToolkitError::Io(e.to_string())
    })?;
    
    file.write_all(content.as_bytes()).map_err(|e| {
        error!("Failed to append to file {:?}: {}", path, e);
        ToolkitError::Io(e.to_string())
    })?;
    
    Ok(())
}

/// Check if a file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists() && path.as_ref().is_file()
}

/// Check if a directory exists
pub fn dir_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists() && path.as_ref().is_dir()
}

/// List all files in a directory (non-recursive)
pub fn list_files(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    debug!("Listing files in directory: {:?}", dir);
    
    if !dir.exists() {
        return Ok(vec![]);
    }
    
    let entries = fs::read_dir(dir).map_err(|e| {
        error!("Failed to read directory {:?}: {}", dir, e);
        ToolkitError::Io(e.to_string())
    })?;
    
    let mut files = Vec::new();
    
    for entry in entries {
        let entry = entry.map_err(|e| {
            error!("Failed to read directory entry: {}", e);
            ToolkitError::Io(e.to_string())
        })?;
        
        let path = entry.path();
        
        if path.is_file() {
            files.push(path);
        }
    }
    
    Ok(files)
}

/// List all directories in a directory (non-recursive)
pub fn list_dirs(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    debug!("Listing directories in directory: {:?}", dir);
    
    if !dir.exists() {
        return Ok(vec![]);
    }
    
    let entries = fs::read_dir(dir).map_err(|e| {
        error!("Failed to read directory {:?}: {}", dir, e);
        ToolkitError::Io(e.to_string())
    })?;
    
    let mut dirs = Vec::new();
    
    for entry in entries {
        let entry = entry.map_err(|e| {
            error!("Failed to read directory entry: {}", e);
            ToolkitError::Io(e.to_string())
        })?;
        
        let path = entry.path();
        
        if path.is_dir() {
            dirs.push(path);
        }
    }
    
    Ok(dirs)
}

/// Find files matching a pattern in a directory (recursive)
pub fn find_files(dir: impl AsRef<Path>, pattern: &str) -> Result<Vec<PathBuf>> {
    use glob::glob_with;
    use glob::MatchOptions;
    
    let dir = dir.as_ref();
    let pattern = if dir.to_string_lossy().is_empty() {
        pattern.to_string()
    } else {
        format!("{}/{}", dir.to_string_lossy(), pattern)
    };
    
    debug!("Finding files with pattern: {}", pattern);
    
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    
    let entries = glob_with(&pattern, options).map_err(|e| {
        error!("Invalid glob pattern '{}': {}", pattern, e);
        ToolkitError::InvalidInput(format!("Invalid glob pattern: {}", e))
    })?;
    
    let mut files = Vec::new();
    
    for entry in entries {
        match entry {
            Ok(path) if path.is_file() => {
                files.push(path);
            }
            Ok(_) => {
                // Not a file, ignore
            }
            Err(e) => {
                error!("Failed to read glob entry: {}", e);
                return Err(ToolkitError::Unknown(format!(
                    "Failed to read glob entry: {}", e
                )));
            }
        }
    }
    
    Ok(files)
}

/// Delete a file if it exists
pub fn delete_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    
    if path.exists() && path.is_file() {
        debug!("Deleting file: {:?}", path);
        fs::remove_file(path).map_err(|e| {
            error!("Failed to delete file {:?}: {}", path, e);
            ToolkitError::Io(e.to_string())
        })?;
    }
    
    Ok(())
}

/// Delete a directory and all its contents
pub fn delete_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    
    if path.exists() && path.is_dir() {
        debug!("Deleting directory: {:?}", path);
        fs::remove_dir_all(path).map_err(|e| {
            error!("Failed to delete directory {:?}: {}", path, e);
            ToolkitError::Io(e.to_string())
        })?;
    }
    
    Ok(())
}

/// Copy a file from source to destination
pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    
    debug!("Copying file from {:?} to {:?}", from, to);
    
    // Ensure parent directory exists
    if let Some(parent) = to.parent() {
        ensure_dir_exists(parent)?;
    }
    
    fs::copy(from, to).map_err(|e| {
        error!("Failed to copy file from {:?} to {:?}: {}", from, to, e);
        ToolkitError::Io(e.to_string())
    })
}

/// Rename a file or directory
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    
    debug!("Renaming {:?} to {:?}", from, to);
    
    // Ensure parent directory exists
    if let Some(parent) = to.parent() {
        ensure_dir_exists(parent)?;
    }
    
    fs::rename(from, to).map_err(|e| {
        error!("Failed to rename {:?} to {:?}: {}", from, to, e);
        ToolkitError::Io(e.to_string())
    })
}

/// Read a file's contents asynchronously
///
/// This function reads the entire contents of a file into a string,
/// using Tokio's async file system operations.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// A Result containing the file's contents as a String, or an error.
pub async fn read_file_async(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    debug!("Reading file: {}", path.display());
    
    tokio_fs::read_to_string(path)
        .await
        .map_err(|e| ToolkitError::Io(e.to_string()))
}

/// Write content to a file asynchronously
///
/// This function writes a string to a file, creating the file if it doesn't exist
/// and overwriting it if it does.
///
/// # Arguments
///
/// * `path` - The path to the file to write
/// * `content` - The content to write to the file
///
/// # Returns
///
/// A Result indicating success or failure.
pub async fn write_file_async(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Writing to file: {}", path.display());
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            tokio_fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolkitError::Io(e.to_string()))?;
        }
    }
    
    tokio_fs::write(path, content)
        .await
        .map_err(|e| ToolkitError::Io(e.to_string()))
}

/// Ensure a directory exists asynchronously
///
/// This function creates a directory and all its parent directories if they don't exist.
///
/// # Arguments
///
/// * `path` - The path to the directory to create
///
/// # Returns
///
/// A Result indicating success or failure.
pub async fn ensure_dir_async(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        debug!("Creating directory: {}", path.display());
        tokio_fs::create_dir_all(path)
            .await
            .map_err(|e| ToolkitError::Io(e.to_string()))?;
    }
    Ok(())
}

// Provide synchronous versions for backward compatibility
/// Read a file's contents synchronously
pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    debug!("Reading file (sync): {}", path.display());
    
    fs::read_to_string(path)
        .map_err(|e| ToolkitError::Io(e.to_string()))
}

/// Write content to a file synchronously
pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Writing to file (sync): {}", path.display());
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolkitError::Io(e.to_string()))?;
        }
    }
    
    fs::write(path, content)
        .map_err(|e| ToolkitError::Io(e.to_string()))
}

/// Ensure a directory exists synchronously
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        debug!("Creating directory (sync): {}", path.display());
        fs::create_dir_all(path)
            .map_err(|e| ToolkitError::Io(e.to_string()))?;
    }
    Ok(())
} 