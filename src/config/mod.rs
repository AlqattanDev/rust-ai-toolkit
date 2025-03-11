// Note: The old providers module has been moved to src/ai
// The empty 'providers' directory should be deleted manually
use crate::error::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::env;
use log::{debug, warn};
use reqwest;
use serde_json;

// Environment variable names for API keys
const ANTHROPIC_API_KEY_ENV: &str = "RUST_AI_TOOLKIT_ANTHROPIC_API_KEY";
const OPENAI_API_KEY_ENV: &str = "RUST_AI_TOOLKIT_OPENAI_API_KEY";
const CUSTOM_API_KEY_ENV: &str = "RUST_AI_TOOLKIT_CUSTOM_API_KEY";

// Extension trait for String and &str to work with colored crate
pub trait ColorizeExt {
    fn green(&self) -> String;
    fn yellow(&self) -> String;
    fn red(&self) -> String;
    fn cyan(&self) -> String;
    fn bright_blue(&self) -> String;
    fn dimmed(&self) -> String;
    fn bold(&self) -> String;
}

impl ColorizeExt for String {
    fn green(&self) -> String {
        self.as_str().green().to_string()
    }
    
    fn yellow(&self) -> String {
        self.as_str().yellow().to_string()
    }
    
    fn red(&self) -> String {
        self.as_str().red().to_string()
    }
    
    fn cyan(&self) -> String {
        self.as_str().cyan().to_string()
    }
    
    fn bright_blue(&self) -> String {
        self.as_str().bright_blue().to_string()
    }
    
    fn dimmed(&self) -> String {
        self.as_str().dimmed().to_string()
    }
    
    fn bold(&self) -> String {
        self.as_str().bold().to_string()
    }
}

impl ColorizeExt for &String {
    fn green(&self) -> String {
        self.as_str().green().to_string()
    }
    
    fn yellow(&self) -> String {
        self.as_str().yellow().to_string()
    }
    
    fn red(&self) -> String {
        self.as_str().red().to_string()
    }
    
    fn cyan(&self) -> String {
        self.as_str().cyan().to_string()
    }
    
    fn bright_blue(&self) -> String {
        self.as_str().bright_blue().to_string()
    }
    
    fn dimmed(&self) -> String {
        self.as_str().dimmed().to_string()
    }
    
    fn bold(&self) -> String {
        self.as_str().bold().to_string()
    }
}

impl ColorizeExt for &str {
    fn green(&self) -> String {
        let s = (*self).to_string();
        s.as_str().green().to_string()
    }
    
    fn yellow(&self) -> String {
        let s = (*self).to_string();
        s.as_str().yellow().to_string()
    }
    
    fn red(&self) -> String {
        let s = (*self).to_string();
        s.as_str().red().to_string()
    }
    
    fn cyan(&self) -> String {
        let s = (*self).to_string();
        s.as_str().cyan().to_string()
    }
    
    fn bright_blue(&self) -> String {
        let s = (*self).to_string();
        s.as_str().bright_blue().to_string()
    }
    
    fn dimmed(&self) -> String {
        let s = (*self).to_string();
        s.as_str().dimmed().to_string()
    }
    
    fn bold(&self) -> String {
        let s = (*self).to_string();
        s.as_str().bold().to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub projects_dir: PathBuf,
    
    // New configuration options
    /// Cache TTL in seconds for project data
    pub project_cache_ttl: u64,
    /// Cache TTL in seconds for AI responses
    pub response_cache_ttl: u64,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u32,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Rate limit settings per minute for each provider
    pub rate_limits: ProviderRateLimits,
}

/// Rate limit settings for different providers
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderRateLimits {
    pub anthropic: u32,
    pub openai: u32,
    pub custom: u32,
}

impl Default for ProviderRateLimits {
    fn default() -> Self {
        Self {
            anthropic: 30,  // 30 requests per minute
            openai: 60,     // 60 requests per minute
            custom: 30,     // 30 requests per minute
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().expect("Could not find home directory");
        let projects_dir = home_dir.join(".rust-ai-toolkit").join("projects");
        
        Self {
            provider: "anthropic".to_string(),
            api_key: "".to_string(),
            base_url: None,
            model: "claude-3-7-sonnet-20250219".to_string(),
            projects_dir,
            
            // Default values for new options
            project_cache_ttl: 3600,        // 1 hour
            response_cache_ttl: 3600,       // 1 hour
            max_cache_size_mb: 1000,        // 1 GB
            log_level: "info".to_string(),
            rate_limits: ProviderRateLimits::default(),
        }
    }
}

/// Masks an API key for logging purposes
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 8 {
        return "[API_KEY_TOO_SHORT]".to_string();
    }
    
    let first_four = &api_key[0..4];
    let last_four = &api_key[api_key.len() - 4..];
    format!("{}...{}", first_four, last_four)
}

/// Get environment variable name for the current provider
fn get_env_var_name(provider: &str) -> &'static str {
    match provider {
        "anthropic" | "anthropic_enhanced" => ANTHROPIC_API_KEY_ENV,
        "openai" => OPENAI_API_KEY_ENV,
        _ => CUSTOM_API_KEY_ENV,
    }
}

pub fn get_config() -> Result<Config> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.toml");
    
    let mut config = if !config_path.exists() {
        Config::default()
    } else {
        // Try to parse the existing config
        let content = fs::read_to_string(&config_path)?;
        match toml::from_str::<Config>(&content) {
            Ok(config) => config,
            Err(e) => {
                // If parsing fails, try to migrate from an older version
                debug!("Failed to parse config, attempting migration: {}", e);
                migrate_config(&content, &config_path)?
            }
        }
    };
    
    // Check for API key in environment variables
    let env_var_name = get_env_var_name(&config.provider);
    if let Ok(api_key) = env::var(env_var_name) {
        if !api_key.is_empty() {
            debug!("Using API key from environment variable: {}", env_var_name);
            config.api_key = api_key;
        }
    } else if !config.api_key.is_empty() {
        // If we're using an API key from config, warn the user
        warn!("Using API key from config file. Consider using environment variable {} for better security.", env_var_name);
    }
    
    Ok(config)
}

/// Migrate from an older config version to the current version
fn migrate_config(content: &str, config_path: &PathBuf) -> Result<Config> {
    use crate::error::ToolkitError;
    
    // Try to parse as a legacy config (without the new fields)
    #[derive(Debug, Serialize, Deserialize)]
    struct LegacyConfig {
        pub provider: String,
        pub api_key: String,
        pub base_url: Option<String>,
        pub model: String,
        pub projects_dir: PathBuf,
    }
    
    let legacy_config = toml::from_str::<LegacyConfig>(content)
        .map_err(|e| ToolkitError::Config(format!("Failed to parse legacy config: {}", e)))?;
    
    // Create a new config with default values for the new fields
    let config = Config {
        provider: legacy_config.provider,
        api_key: legacy_config.api_key,
        base_url: legacy_config.base_url,
        model: legacy_config.model,
        projects_dir: legacy_config.projects_dir,
        project_cache_ttl: 3600,        // 1 hour
        response_cache_ttl: 3600,       // 1 hour
        max_cache_size_mb: 1000,        // 1 GB
        log_level: "info".to_string(),
        rate_limits: ProviderRateLimits::default(),
    };
    
    // Save the migrated config
    save_config(&config)?;
    
    println!("{}", "Your configuration has been migrated to the new format with default values for new settings.".yellow());
    println!("{}", "You can update these settings by running 'rust-ai-toolkit config' again.".yellow());
    
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.toml");
    
    // Create projects directory if it doesn't exist
    if !config.projects_dir.exists() {
        fs::create_dir_all(&config.projects_dir)?;
    }
    
    let content = toml::to_string(config).map_err(|e| {
        crate::error::ToolkitError::Config(format!("Failed to serialize config: {}", e))
    })?;
    
    fs::write(config_path, content)?;
    
    // Inform the user about environment variables for API keys
    let env_var_name = get_env_var_name(&config.provider);
    println!("{}", format!("Configuration saved successfully. For better security, consider setting your API key via the {} environment variable instead of storing it in the config file.", env_var_name).yellow());
    
    Ok(())
}

fn get_config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        crate::error::ToolkitError::Config("Could not find home directory".to_string())
    })?;
    
    let config_dir = home_dir.join(".rust-ai-toolkit");
    
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    
    Ok(config_dir)
}

/// Enum representing the configuration steps
enum ConfigStep {
    Provider,
    ApiKey,
    Model,
    BaseUrl,
    RateLimits,
    CacheTTL,
    CacheSize,
    LogLevel,
    Confirmation,
}

impl ConfigStep {
    /// Get the next step
    fn next(&self) -> ConfigStep {
        match self {
            ConfigStep::Provider => ConfigStep::ApiKey,
            ConfigStep::ApiKey => ConfigStep::Model,
            ConfigStep::Model => ConfigStep::BaseUrl,
            ConfigStep::BaseUrl => ConfigStep::RateLimits,
            ConfigStep::RateLimits => ConfigStep::CacheTTL,
            ConfigStep::CacheTTL => ConfigStep::CacheSize,
            ConfigStep::CacheSize => ConfigStep::LogLevel,
            ConfigStep::LogLevel => ConfigStep::Confirmation,
            ConfigStep::Confirmation => ConfigStep::Confirmation,
        }
    }
    
    /// Get the previous step
    fn prev(&self) -> ConfigStep {
        match self {
            ConfigStep::Provider => ConfigStep::Provider,
            ConfigStep::ApiKey => ConfigStep::Provider,
            ConfigStep::Model => ConfigStep::ApiKey,
            ConfigStep::BaseUrl => ConfigStep::Model,
            ConfigStep::RateLimits => ConfigStep::BaseUrl,
            ConfigStep::CacheTTL => ConfigStep::RateLimits,
            ConfigStep::CacheSize => ConfigStep::CacheTTL,
            ConfigStep::LogLevel => ConfigStep::CacheSize,
            ConfigStep::Confirmation => ConfigStep::LogLevel,
        }
    }
}

pub async fn configure_ai() -> Result<()> {
    use dialoguer::{Input, Password, Confirm};
    use colored::Colorize;
    
    // Load current configuration
    let mut config = get_config()?;
    let mut current_step = ConfigStep::Provider;
    let theme = ColorfulTheme::default();
    
    // Main configuration loop
    loop {
        match current_step {
            ConfigStep::Provider => {
                // Display header
                println!("\n{}\n", "AI Provider Configuration".green().bold());
                
                // Help text
                println!("{}", "Select which AI provider you want to use.".cyan());
                println!("{}\n", "This determines which API will be used for AI interactions.".cyan());
                
                // Show current value if any
                if !config.provider.is_empty() {
                    println!("Current provider: {}", config.provider.yellow());
                }
                
                // Choose provider
                let providers = vec!["Anthropic (Claude)", "Anthropic Enhanced (Claude Code)", "OpenAI", "Custom API"];
                let provider_idx = Select::with_theme(&theme)
                    .with_prompt("Select AI provider")
                    .default(match config.provider.as_str() {
                        "anthropic" => 0,
                        "anthropic_enhanced" => 1,
                        "openai" => 2,
                        "custom" => 3,
                        _ => 0,
                    })
                    .items(&providers)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                config.provider = match provider_idx {
                    0 => "anthropic".to_string(),
                    1 => "anthropic_enhanced".to_string(),
                    2 => "openai".to_string(),
                    3 => "custom".to_string(),
                    _ => "anthropic".to_string(),
                };
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::ApiKey => {
                // Display header
                println!("\n{}\n", "API Key Configuration".green().bold());
                
                // Show API key format info based on provider
                match config.provider.as_str() {
                    "anthropic" | "anthropic_enhanced" => {
                        println!("{}", "Anthropic API keys typically start with 'sk-ant-'.".cyan());
                        println!("{}\n", "You can find your API key in the Anthropic Console: https://console.anthropic.com/".cyan());
                    },
                    "openai" => {
                        println!("{}", "OpenAI API keys typically start with 'sk-'.".cyan());
                        println!("{}\n", "You can find your API key in the OpenAI dashboard: https://platform.openai.com/api-keys".cyan());
                    },
                    _ => {
                        println!("{}\n", "Enter the API key for your custom provider.".cyan());
                    }
                }
                
                // Show current value if any (masked)
                if !config.api_key.is_empty() {
                    println!("Current API key: {}\n", mask_api_key(&config.api_key).yellow());
                }
                
                // Get environment variable name
                let env_var_name = get_env_var_name(&config.provider);
                println!("{}", format!("You can also set this via the {} environment variable.", env_var_name).cyan());
                
                // Let user know about security
                println!("{}\n", "For better security, using environment variables is recommended.".cyan());
                
                // Configure API key
                let api_key = Password::with_theme(&theme)
                    .with_prompt("Enter your API key")
                    .allow_empty_password(true)  // Allow empty to keep current
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                // Validate API key format
                if !api_key.is_empty() {
                    if !is_valid_api_key_format(&api_key, &config.provider) {
                        println!("{}", "Warning: API key format appears to be invalid.".red());
                        if !Confirm::with_theme(&theme)
                            .with_prompt("Do you want to use this API key anyway?")
                            .default(false)
                            .interact()
                            .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))? {
                            // Stay on this step if they want to try again
                            continue;
                        }
                    }
                    
                    // Update the config if a new key was provided
                    config.api_key = api_key;
                } else if config.api_key.is_empty() {
                    println!("{}", "No API key provided. You'll need to set one via environment variable.".yellow());
                }
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::Model => {
                // Display header
                println!("\n{}\n", "Model Configuration".green().bold());
                
                // Help text
                println!("{}", "Select the AI model to use.".cyan());
                println!("{}\n", "Different models have different capabilities and pricing.".cyan());
                
                // Show current value if any
                if !config.model.is_empty() {
                    println!("Current model: {}\n", config.model.yellow());
                }
                
                // Configure model based on provider
                let models = match config.provider.as_str() {
                    "anthropic" => vec![
                        "claude-3-7-sonnet-20250219",
                        "claude-3-5-sonnet-v2-20241022",
                        "claude-3-5-sonnet-20240620",
                        "claude-3-opus-20240229",
                        "claude-3-sonnet-20240229",
                        "claude-3-haiku-20240307",
                    ],
                    "anthropic_enhanced" => vec![
                        "claude-3-7-sonnet-20250219",
                        "claude-3-5-sonnet-v2-20241022",
                        "claude-3-5-sonnet-20240620",
                        "claude-3-opus-20240229",
                        "claude-3-sonnet-20240229",
                        "claude-3-haiku-20240307",
                    ],
                    "openai" => vec![
                        "gpt-4o-2024-05-13",
                        "gpt-4-turbo-2024-04-09",
                        "gpt-4o",
                        "gpt-4-turbo",
                        "gpt-4",
                        "gpt-3.5-turbo",
                    ],
                    _ => vec!["custom-model"],
                };
                
                let default_idx = models.iter().position(|&m| m == config.model).unwrap_or(0);
                
                let model_idx = Select::with_theme(&theme)
                    .with_prompt("Select model")
                    .default(default_idx)
                    .items(&models)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                config.model = models[model_idx].to_string();
                
                // Or allow custom input for model
                if config.provider == "custom" {
                    let custom_model = Input::<String>::with_theme(&theme)
                        .with_prompt("Or enter a custom model name")
                        .allow_empty(true)
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                    
                    if !custom_model.is_empty() {
                        config.model = custom_model;
                    }
                }
                
                // Validate model
                if !is_valid_model(&config.model, &config.provider) {
                    println!("{}", "Warning: The selected model may not be compatible with the provider.".red());
                    
                    if !Confirm::with_theme(&theme)
                        .with_prompt("Do you want to use this model anyway?")
                        .default(false)
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))? {
                        // Stay on this step if they want to try again
                        continue;
                    }
                }
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::BaseUrl => {
                // Display header
                println!("\n{}\n", "Base URL Configuration".green().bold());
                
                // Help text
                println!("{}", "Set a custom base URL for the API (if needed).".cyan());
                println!("{}\n", "This is only required for custom deployments or proxies.".cyan());
                
                // Show current value if any
                if let Some(url) = &config.base_url {
                    println!("Current base URL: {}\n", url.yellow());
                } else {
                    println!("Current base URL: {}\n", "Using default".yellow());
                }
                
                // Default URLs based on provider
                let default_url = match config.provider.as_str() {
                    "anthropic" | "anthropic_enhanced" => "https://api.anthropic.com/v1",
                    "openai" => "https://api.openai.com/v1",
                    _ => "",
                };
                
                // Ask if they want to use a custom base URL
                let use_custom_url = config.provider == "custom" || 
                    Confirm::with_theme(&theme)
                        .with_prompt("Do you want to use a custom base URL?")
                        .default(config.base_url.is_some())
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))?;
                
                if use_custom_url {
                    let base_url = Input::<String>::with_theme(&theme)
                        .with_prompt("Enter base URL for API")
                        .with_initial_text(config.base_url.clone().unwrap_or_else(|| default_url.to_string()))
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                    
                    // Validate URL
                    if !is_valid_url(&base_url) {
                        println!("{}", "Warning: The URL format appears to be invalid.".red());
                        println!("{}", "URLs should start with http:// or https://".red());
                        
                        if !Confirm::with_theme(&theme)
                            .with_prompt("Do you want to use this URL anyway?")
                            .default(false)
                            .interact()
                            .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))? {
                            // Stay on this step if they want to try again
                            continue;
                        }
                    }
                    
                    config.base_url = Some(base_url);
                } else {
                    config.base_url = None;
                    println!("Using default base URL for {}", config.provider);
                }
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::RateLimits => {
                // Display header
                println!("\n{}\n", "Rate Limit Configuration".green().bold());
                
                // Help text
                println!("{}", "Configure rate limits for API requests.".cyan());
                println!("{}\n", "This helps prevent hitting provider rate limits.".cyan());
                
                // Show current values
                println!("Current rate limits (requests per minute):");
                println!("  - Anthropic: {}", config.rate_limits.anthropic.to_string().yellow());
                println!("  - OpenAI: {}", config.rate_limits.openai.to_string().yellow());
                println!("  - Custom: {}\n", config.rate_limits.custom.to_string().yellow());
                
                // Configure rate limits for each provider
                println!("Configure rate limits for each provider (requests per minute):");
                
                let anthropic_rate = Input::<u32>::with_theme(&theme)
                    .with_prompt("Anthropic rate limit")
                    .with_initial_text(config.rate_limits.anthropic.to_string())
                    .validate_with(|input: &u32| {
                        if is_valid_rate_limit(*input) {
                            Ok(())
                        } else {
                            Err("Rate limit must be between 1 and 1000")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                let openai_rate = Input::<u32>::with_theme(&theme)
                    .with_prompt("OpenAI rate limit")
                    .with_initial_text(config.rate_limits.openai.to_string())
                    .validate_with(|input: &u32| {
                        if is_valid_rate_limit(*input) {
                            Ok(())
                        } else {
                            Err("Rate limit must be between 1 and 1000")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                let custom_rate = Input::<u32>::with_theme(&theme)
                    .with_prompt("Custom provider rate limit")
                    .with_initial_text(config.rate_limits.custom.to_string())
                    .validate_with(|input: &u32| {
                        if is_valid_rate_limit(*input) {
                            Ok(())
                        } else {
                            Err("Rate limit must be between 1 and 1000")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                config.rate_limits.anthropic = anthropic_rate;
                config.rate_limits.openai = openai_rate;
                config.rate_limits.custom = custom_rate;
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::CacheTTL => {
                // Display header
                println!("\n{}\n", "Cache TTL Configuration".green().bold());
                
                // Help text
                println!("{}", "Configure cache time-to-live (TTL) settings.".cyan());
                println!("{}\n", "This determines how long cached data is considered valid.".cyan());
                
                // Show current values
                println!("Current TTL settings (in seconds):");
                println!("  - Project cache: {}", config.project_cache_ttl.to_string().yellow());
                println!("  - Response cache: {}\n", config.response_cache_ttl.to_string().yellow());
                
                // Configure cache TTLs
                let project_ttl = Input::<u64>::with_theme(&theme)
                    .with_prompt("Project cache TTL (seconds)")
                    .with_initial_text(config.project_cache_ttl.to_string())
                    .validate_with(|input: &u64| {
                        if is_valid_ttl(*input) {
                            Ok(())
                        } else {
                            Err("TTL must be greater than 0")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                let response_ttl = Input::<u64>::with_theme(&theme)
                    .with_prompt("Response cache TTL (seconds)")
                    .with_initial_text(config.response_cache_ttl.to_string())
                    .validate_with(|input: &u64| {
                        if is_valid_ttl(*input) {
                            Ok(())
                        } else {
                            Err("TTL must be greater than 0")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                config.project_cache_ttl = project_ttl;
                config.response_cache_ttl = response_ttl;
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::CacheSize => {
                // Display header
                println!("\n{}\n", "Cache Size Configuration".green().bold());
                
                // Help text
                println!("{}", "Configure maximum cache size.".cyan());
                println!("{}\n", "This limits how much disk space the cache can use.".cyan());
                
                // Show current value
                println!("Current maximum cache size: {} MB\n", config.max_cache_size_mb.to_string().yellow());
                
                // Configure cache size
                let cache_size = Input::<u32>::with_theme(&theme)
                    .with_prompt("Maximum cache size (MB)")
                    .with_initial_text(config.max_cache_size_mb.to_string())
                    .validate_with(|input: &u32| {
                        if is_valid_cache_size(*input) {
                            Ok(())
                        } else {
                            Err("Cache size must be between 1 and 10000 MB")
                        }
                    })
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Input error: {}", e)))?;
                
                config.max_cache_size_mb = cache_size;
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::LogLevel => {
                // Display header
                println!("\n{}\n", "Log Level Configuration".green().bold());
                
                // Help text
                println!("{}", "Configure the logging verbosity level.".cyan());
                println!("{}\n", "More verbose levels (debug, trace) provide more information but create larger log files.".cyan());
                
                // Show current value
                println!("Current log level: {}\n", config.log_level.yellow());
                
                // Configure log level
                let log_levels = vec!["error", "warn", "info", "debug", "trace"];
                let current_idx = log_levels.iter().position(|&l| l == config.log_level).unwrap_or(2); // Default to info
                
                let level_idx = Select::with_theme(&theme)
                    .with_prompt("Select log level")
                    .default(current_idx)
                    .items(&log_levels)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                config.log_level = log_levels[level_idx].to_string();
                
                // Navigation options
                let actions = vec!["Continue", "Back"];
                let action_idx = Select::with_theme(&theme)
                    .with_prompt("What would you like to do?")
                    .default(0)
                    .items(&actions)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                
                current_step = match action_idx {
                    0 => current_step.next(),
                    1 => current_step.prev(),
                    _ => current_step.next(),
                };
            },
            
            ConfigStep::Confirmation => {
                // Display header
                println!("\n{}\n", "Configuration Summary".green().bold());
                
                // Show summary of configuration
                println!("Provider: {}", config.provider.yellow());
                println!("API Key: {}", mask_api_key(&config.api_key).yellow());
                println!("Model: {}", config.model.yellow());
                println!("Base URL: {}", config.base_url.clone().unwrap_or_else(|| "default".to_string()).yellow());
                println!("\nRate limits (requests per minute):");
                println!("  - Anthropic: {}", config.rate_limits.anthropic.to_string().yellow());
                println!("  - OpenAI: {}", config.rate_limits.openai.to_string().yellow());
                println!("  - Custom: {}", config.rate_limits.custom.to_string().yellow());
                println!("\nCache settings:");
                println!("  - Project cache TTL: {} seconds", config.project_cache_ttl.to_string().yellow());
                println!("  - Response cache TTL: {} seconds", config.response_cache_ttl.to_string().yellow());
                println!("  - Maximum cache size: {} MB", config.max_cache_size_mb.to_string().yellow());
                println!("\nLog level: {}", config.log_level.yellow());
                
                // Ask if they want to validate the API key
                let validate_key = if !config.api_key.is_empty() {
                    Confirm::with_theme(&theme)
                        .with_prompt("Would you like to validate your API key by making a test request?")
                        .default(true)
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))?
                } else {
                    false
                };
                
                if validate_key {
                    println!("Validating API key with a test request...");
                    match test_api_key(&config.provider, &config.api_key, &config.model, config.base_url.clone()).await {
                        Ok(_) => {
                            println!("{}", "API key validation successful!".green());
                        },
                        Err(e) => {
                            println!("{}", format!("API key validation failed: {}", e).red());
                            println!("{}", "You can still save this configuration, but it may not work correctly.".yellow());
                            
                            if !Confirm::with_theme(&theme)
                                .with_prompt("Do you want to go back and fix the API key?")
                                .default(true)
                                .interact()
                                .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))? {
                                // Continue to save if they don't want to fix
                            } else {
                                current_step = ConfigStep::ApiKey;
                                continue;
                            }
                        }
                    }
                }
                
                // Ask for confirmation to save
                let should_save = Confirm::with_theme(&theme)
                    .with_prompt("Save this configuration?")
                    .default(true)
                    .interact()
                    .map_err(|e| crate::error::ToolkitError::Config(format!("Confirmation error: {}", e)))?;
                
                if should_save {
                    // Save configuration
                    save_config(&config)?;
                    
                    println!("\n{}", "Configuration saved successfully.".green());
                    
                    // Configure rate limiter with new settings
                    crate::utils::rate_limiter::set_rate_limit("anthropic", config.rate_limits.anthropic);
                    crate::utils::rate_limiter::set_rate_limit("openai", config.rate_limits.openai);
                    crate::utils::rate_limiter::set_rate_limit("custom", config.rate_limits.custom);
                    
                    break; // Exit the loop
                } else {
                    // Navigation options
                    let actions = vec!["Start over", "Go back to edit", "Exit without saving"];
                    let action_idx = Select::with_theme(&theme)
                        .with_prompt("What would you like to do?")
                        .default(1)
                        .items(&actions)
                        .interact()
                        .map_err(|e| crate::error::ToolkitError::Config(format!("Selection error: {}", e)))?;
                    
                    match action_idx {
                        0 => current_step = ConfigStep::Provider, // Start over
                        1 => current_step = current_step.prev(),  // Go back
                        _ => break,                              // Exit without saving
                    }
                }
            },
        }
    }
    
    Ok(())
}

/// Validates a URL string.
///
/// # Parameters
///
/// * `url` - The URL string to validate.
///
/// # Returns
///
/// `true` if the URL is valid, `false` otherwise.
fn is_valid_url(url: &str) -> bool {
    // Simple check for URL format
    url.starts_with("http://") || url.starts_with("https://")
}

/// Validates an API key format based on the provider.
///
/// # Parameters
///
/// * `api_key` - The API key to validate.
/// * `provider` - The provider name (anthropic, openai, custom).
///
/// # Returns
///
/// `true` if the API key format is valid, `false` otherwise.
fn is_valid_api_key_format(api_key: &str, provider: &str) -> bool {
    if api_key.is_empty() {
        return false;
    }
    
    match provider {
        "anthropic" | "anthropic_enhanced" => {
            // Anthropic API keys typically start with sk-ant-
            // But we'll accept keys that are at least 8 characters for flexibility
            api_key.len() >= 8
        }
        "openai" => {
            // OpenAI API keys typically start with sk-
            // But we'll accept keys that are at least 8 characters for flexibility
            api_key.len() >= 8
        }
        _ => api_key.len() >= 8, // Minimum length for any API key
    }
}

/// Validates that a model name is supported for the given provider.
///
/// # Parameters
///
/// * `model` - The model name to validate.
/// * `provider` - The provider name (anthropic, openai, custom).
///
/// # Returns
///
/// `true` if the model is valid for the provider, `false` otherwise.
fn is_valid_model(model: &str, provider: &str) -> bool {
    match provider {
        "anthropic" | "anthropic_enhanced" => {
            // Anthropic models
            model.contains("claude")
        }
        "openai" => {
            // OpenAI models
            model.contains("gpt")
        }
        _ => true, // For custom providers, accept any model name
    }
}

/// Validates a log level string.
///
/// # Parameters
///
/// * `level` - The log level string to validate.
///
/// # Returns
///
/// `true` if the log level is valid, `false` otherwise.
fn is_valid_log_level(level: &str) -> bool {
    matches!(level, "trace" | "debug" | "info" | "warn" | "error")
}

/// Validates a TTL value in seconds.
///
/// # Parameters
///
/// * `ttl` - The TTL value to validate.
///
/// # Returns
///
/// `true` if the TTL is valid (greater than 0), `false` otherwise.
fn is_valid_ttl(ttl: u64) -> bool {
    ttl > 0
}

/// Validates a cache size value in MB.
///
/// # Parameters
///
/// * `size_mb` - The cache size in MB to validate.
///
/// # Returns
///
/// `true` if the cache size is valid (between 1 and 10000), `false` otherwise.
fn is_valid_cache_size(size_mb: u32) -> bool {
    size_mb >= 1 && size_mb <= 10000
}

/// Validates a rate limit value.
///
/// # Parameters
///
/// * `rate_limit` - The rate limit value to validate.
///
/// # Returns
///
/// `true` if the rate limit is valid (between 1 and 1000), `false` otherwise.
fn is_valid_rate_limit(rate_limit: u32) -> bool {
    rate_limit >= 1 && rate_limit <= 1000
}

/// Tests API key validity by making a test request to the provider's API.
///
/// # Parameters
///
/// * `provider` - The provider name.
/// * `api_key` - The API key to test.
/// * `model` - The model to use for the test.
/// * `base_url` - Optional base URL for the API.
///
/// # Returns
///
/// `Ok(())` if the API key is valid, an error otherwise.
async fn test_api_key(provider: &str, api_key: &str, model: &str, base_url: Option<String>) -> Result<()> {
    use crate::error::ToolkitError;
    
    // Create a temporary config with the provided values
    let mut temp_config = Config::default();
    temp_config.provider = provider.to_string();
    temp_config.api_key = api_key.to_string();
    temp_config.model = model.to_string();
    temp_config.base_url = base_url.clone(); // Clone here to avoid move
    
    // Temporarily save the config
    let config_dir = get_config_dir()?;
    let temp_config_path = config_dir.join("temp_config.toml");
    
    let content = toml::to_string(&temp_config).map_err(|e| {
        ToolkitError::Config(format!("Failed to serialize config: {}", e))
    })?;
    
    fs::write(&temp_config_path, content)?;
    
    // Create a test client with minimal capabilities
    let result = async {
        // Make a simple test request
        let _options = crate::ai::RequestOptions {
            max_tokens: Some(10),
            temperature: Some(0.0),
            top_p: None,
            timeout: Some(std::time::Duration::from_secs(10)),
            functions: None,
        };
        
        // Create a minimal HTTP client to test the API key
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| ToolkitError::Network(e.to_string()))?;
        
        // Different API endpoints and request structures based on provider
        match provider {
            "anthropic" | "anthropic_enhanced" => {
                // Anthropic API test
                let url = base_url.clone().unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
                
                let request_body = serde_json::json!({
                    "model": model,
                    "max_tokens": 10,
                    "messages": [
                        {"role": "user", "content": "test"}
                    ]
                });
                
                let response = client
                    .post(&url)
                    .header("X-Api-Key", api_key)
                    .header("anthropic-version", "2024-02-15")
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(|e| ToolkitError::Network(e.to_string()))?;
                
                // Capture status before consuming response with text()
                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(ToolkitError::Api(format!(
                        "API key validation failed ({}): {}",
                        status,
                        error_text
                    )));
                }
            },
            "openai" => {
                // OpenAI API test
                let url = base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
                
                let request_body = serde_json::json!({
                    "model": model,
                    "max_tokens": 10,
                    "messages": [
                        {"role": "user", "content": "test"}
                    ]
                });
                
                let response = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(|e| ToolkitError::Network(e.to_string()))?;
                
                // Capture status before consuming response with text()
                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(ToolkitError::Api(format!(
                        "API key validation failed ({}): {}",
                        status,
                        error_text
                    )));
                }
            },
            _ => {
                return Err(ToolkitError::Config(format!("Unsupported provider for validation: {}", provider)));
            }
        }
        
        Ok(())
    }.await;
    
    // Clean up temporary config
    if temp_config_path.exists() {
        let _ = fs::remove_file(temp_config_path);
    }
    
    result
}