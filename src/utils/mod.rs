pub mod project;
pub mod rate_limiter;
pub mod cache;
pub mod file;
pub mod ui;

/// Logging utilities for consistent output formatting
pub mod logging {
    use colored::Colorize;
    use log::{debug, error, info, warn};

    /// Log an informational message to both the log file and stdout
    pub fn info_user(message: &str) {
        info!("{}", message);
        println!("{}", message);
    }

    /// Log a success message to both the log file and stdout
    pub fn success(message: &str) {
        info!("SUCCESS: {}", message);
        println!("{}", message.green());
    }

    /// Log a warning message to both the log file and stdout
    pub fn warn_user(message: &str) {
        warn!("{}", message);
        println!("{}", message.yellow());
    }

    /// Log an error message to both the log file and stdout
    pub fn error_user(message: &str) {
        error!("{}", message);
        println!("{}", message.red());
    }

    /// Log a debug message to both the log file and stdout if in debug mode
    pub fn debug_user(message: &str) {
        debug!("{}", message);
        if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
            println!("{}", format!("DEBUG: {}", message).dimmed());
        }
    }
}

// Re-export commonly used utilities for easier imports
