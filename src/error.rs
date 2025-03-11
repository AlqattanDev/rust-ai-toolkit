//! Error handling for the Rust AI Toolkit.
//!
//! This module defines the error types and helper functions used throughout the toolkit.
//! It provides a consistent way to handle and report errors to users.
//!
//! The main components are:
//! - [`ToolkitError`]: The main error enum used throughout the application
//! - [`Result<T>`]: A type alias for `std::result::Result<T, ToolkitError>`
//! - Conversion implementations from common error types to `ToolkitError`
//!
//! # Examples
//!
//! ```
//! use crate::error::{Result, ToolkitError};
//!
//! fn example_function() -> Result<()> {
//!     // Return a specific error
//!     if something_went_wrong {
//!         return Err(ToolkitError::InvalidInput("Invalid parameter".to_string()));
//!     }
//!
//!     // Use the ? operator with functions that return std::io::Error
//!     let file = std::fs::File::open("config.toml")?;
//!
//!     Ok(())
//! }
//! ```

use thiserror::Error;
use colored::Colorize;

/// The main error type for the Rust AI Toolkit.
///
/// This enum represents all possible errors that can occur in the toolkit.
/// Each variant includes a descriptive error message and, where appropriate,
/// suggestions for how to resolve the issue.
#[derive(Error, Debug, Clone)]
pub enum ToolkitError {
    /// I/O errors, such as file not found or permission denied.
    #[error("IO error: {0}. Check file permissions and disk space.")]
    Io(String),
    
    /// API-related errors, such as authentication failures or invalid requests.
    #[error("API error: {0}. Please check your API key and network connection.")]
    Api(String),
    
    /// Configuration errors, such as missing or invalid configuration values.
    #[error("Configuration error: {0}. Try running 'rust-ai-toolkit config' to reconfigure.")]
    Config(String),
    
    /// Errors when a requested project cannot be found.
    #[error("Project not found: {0}. Check the project ID or look in the configured projects directory.")]
    ProjectNotFound(String),
    
    /// Errors when a requested stage is invalid.
    #[error("Stage not found: {0}. Stages must be between 1 and 6.")]
    StageNotFound(u8),
    
    /// Network-related errors, such as connection failures or timeouts.
    #[error("Network error: {0}. Please check your internet connection and try again.")]
    Network(String),
    
    /// Serialization or deserialization errors.
    #[error("Serialization error: {0}. The file might be corrupted or in an invalid format.")]
    Serialization(String),
    
    /// File-related errors, such as file not found or permission denied.
    #[error("File error: {0}. The file might not exist or you don't have permission to access it.")]
    File(String),
    
    /// Invalid input errors, such as invalid command-line arguments.
    #[error("Invalid input: {0}. Please check your input and try again.")]
    InvalidInput(String),
    
    /// Template-related errors, such as invalid template syntax.
    #[error("Template error: {0}. There was an issue with template rendering or loading.")]
    TemplateError(String),
    
    /// Parsing errors, such as invalid JSON or TOML.
    #[error("Parse error: {0}. Failed to parse response or data.")]
    Parse(String),
    
    /// Rate limit exceeded errors.
    #[error("Rate limit exceeded: {0}. Please wait before making more requests.")]
    RateLimit(String),
    
    /// Unknown or unexpected errors.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Colorize an error message for display in the terminal.
///
/// This function takes a `ToolkitError` and returns a colorized string
/// representation that can be displayed to the user.
///
/// # Parameters
///
/// * `err` - The error to colorize.
///
/// # Returns
///
/// A colorized string representation of the error.
pub fn colorize_error(err: &ToolkitError) -> String {
    err.to_string().red().to_string()
}

/// A type alias for `std::result::Result<T, ToolkitError>`.
///
/// This is the standard result type used throughout the toolkit.
pub type Result<T> = std::result::Result<T, ToolkitError>;

// Implement From for handlebars::TemplateError
impl From<handlebars::TemplateError> for ToolkitError {
    fn from(err: handlebars::TemplateError) -> Self {
        ToolkitError::TemplateError(err.to_string())
    }
}

// Implement From for handlebars::RenderError
impl From<handlebars::RenderError> for ToolkitError {
    fn from(err: handlebars::RenderError) -> Self {
        ToolkitError::TemplateError(err.to_string())
    }
}

// Update the From implementations to handle the new String-based variants
impl From<std::io::Error> for ToolkitError {
    fn from(err: std::io::Error) -> Self {
        ToolkitError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for ToolkitError {
    fn from(err: reqwest::Error) -> Self {
        ToolkitError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for ToolkitError {
    fn from(err: serde_json::Error) -> Self {
        ToolkitError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for ToolkitError {
    fn from(err: toml::de::Error) -> Self {
        ToolkitError::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for ToolkitError {
    fn from(err: toml::ser::Error) -> Self {
        ToolkitError::Serialization(err.to_string())
    }
}

// Add more From implementations as needed
