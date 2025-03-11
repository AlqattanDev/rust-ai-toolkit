//! Template management for AI prompts.
//!
//! This module provides functionality for loading, rendering, and managing
//! prompt templates. It uses the Handlebars templating engine to support
//! variable substitution and conditional logic in templates.
//!
//! The main components are:
//! - [`PromptManager`]: The core template management struct
//! - Default templates for common AI interactions
//! - Utility functions for template rendering
//!
//! # Examples
//!
//! ```no_run
//! use crate::prompts::PromptManager;
//! use crate::error::Result;
//! use std::collections::HashMap;
//! use std::path::Path;
//!
//! fn example() -> Result<()> {
//!     // Create a new prompt manager
//!     let mut manager = PromptManager::new(Path::new("./templates"))?;
//!     
//!     // Create template variables
//!     let mut vars = HashMap::new();
//!     vars.insert("project_name".to_string(), "My Project".to_string());
//!     vars.insert("description".to_string(), "A sample project".to_string());
//!     
//!     // Convert variables to JSON for rendering
//!     let data = PromptManager::vars_to_json(vars);
//!     
//!     // Render a template
//!     let rendered = manager.render("stage1", &data)?;
//!     println!("{}", rendered);
//!     
//!     // Add a new template
//!     manager.add_template("custom", "This is a {{project_name}} template.")?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Template Format
//!
//! Templates use Handlebars syntax for variable substitution and logic:
//! - `{{variable_name}}` - Inserts the value of a variable
//! - `{{#if condition}}...{{else}}...{{/if}}` - Conditional blocks
//! - `{{#each items}}...{{/each}}` - Iteration over arrays
//!
//! See the Handlebars documentation for more details on the template syntax.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use handlebars::Handlebars;
use serde_json::Value;
use log::{debug, error};
use crate::error::{Result, ToolkitError};

/// A prompt template manager that handles loading and rendering templates.
///
/// This struct provides methods for loading templates from a directory,
/// rendering templates with variable substitution, and managing the template
/// lifecycle.
///
/// # Examples
///
/// ```no_run
/// use crate::prompts::PromptManager;
/// use crate::error::Result;
/// use serde_json::json;
/// use std::path::Path;
///
/// fn example() -> Result<()> {
///     // Create a new prompt manager
///     let manager = PromptManager::new(Path::new("./templates"))?;
///     
///     // Render a template with variables
///     let data = json!({
///         "project_name": "My Project",
///         "description": "A sample project"
///     });
///     
///     let rendered = manager.render("stage1", &data)?;
///     println!("{}", rendered);
///     
///     Ok(())
/// }
/// ```
pub struct PromptManager {
    /// The Handlebars template engine instance.
    handlebars: Handlebars<'static>,
    /// The directory where templates are stored.
    template_dir: PathBuf,
}

impl PromptManager {
    /// Create a new PromptManager with the given template directory.
    ///
    /// This constructor initializes a new prompt manager, creates the template
    /// directory if it doesn't exist, and loads any existing templates from the
    /// directory. It also registers default templates as fallbacks.
    ///
    /// # Parameters
    ///
    /// * `template_dir` - The directory where templates are stored.
    ///
    /// # Returns
    ///
    /// A `Result` containing the new `PromptManager` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the template directory cannot be created or if
    /// templates cannot be loaded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    /// use std::path::Path;
    ///
    /// let manager = PromptManager::new(Path::new("./templates")).unwrap();
    /// ```
    pub fn new(template_dir: impl AsRef<Path>) -> Result<Self> {
        let template_dir = template_dir.as_ref().to_path_buf();
        
        // Ensure the template directory exists
        if !template_dir.exists() {
            fs::create_dir_all(&template_dir)?;
        }
        
        let mut handlebars = Handlebars::new();
        // Don't escape HTML entities in the templates
        handlebars.set_strict_mode(false);
        
        // Load all templates from the template directory
        Self::load_templates(&mut handlebars, &template_dir)?;
        
        // Register default templates as fallbacks
        Self::register_default_templates(&mut handlebars);
        
        Ok(Self {
            handlebars,
            template_dir,
        })
    }
    
    /// Load all templates from the template directory.
    ///
    /// This method scans the template directory for `.hbs` files and registers
    /// them with the Handlebars engine.
    ///
    /// # Parameters
    ///
    /// * `handlebars` - The Handlebars engine to register templates with.
    /// * `template_dir` - The directory to scan for templates.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or if a template
    /// cannot be registered.
    fn load_templates(handlebars: &mut Handlebars, template_dir: &Path) -> Result<()> {
        debug!("Loading templates from {:?}", template_dir);
        
        if !template_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(template_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "hbs") {
                let template_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| ToolkitError::InvalidInput(format!("Invalid template name: {:?}", path)))?;
                
                debug!("Loading template: {}", template_name);
                handlebars.register_template_file(template_name, &path)?;
            }
        }
        
        Ok(())
    }
    
    /// Register default templates as fallbacks.
    ///
    /// This method registers the built-in default templates that are used as
    /// fallbacks when a requested template is not found in the template directory.
    ///
    /// # Parameters
    ///
    /// * `handlebars` - The Handlebars engine to register templates with.
    fn register_default_templates(handlebars: &mut Handlebars) {
        for (name, content) in templates::DEFAULT_TEMPLATES.iter() {
            // Only register if not already registered
            if !handlebars.has_template(name) {
                debug!("Registering default template: {}", name);
                handlebars.register_template_string(name, content).unwrap_or_else(|e| {
                    error!("Failed to register default template {}: {}", name, e);
                });
            }
        }
    }
    
    /// Render a template with the given data.
    ///
    /// This method renders a template with the provided data, substituting
    /// variables and processing any template logic.
    ///
    /// # Parameters
    ///
    /// * `template_name` - The name of the template to render.
    /// * `data` - The data to use for variable substitution.
    ///
    /// # Returns
    ///
    /// A `Result` containing the rendered template as a string if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be found or if rendering fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    /// use serde_json::json;
    /// use std::path::Path;
    ///
    /// let manager = PromptManager::new(Path::new("./templates")).unwrap();
    /// let data = json!({
    ///     "project_name": "My Project",
    ///     "description": "A sample project"
    /// });
    ///
    /// let rendered = manager.render("stage1", &data).unwrap();
    /// println!("{}", rendered);
    /// ```
    pub fn render(&self, template_name: &str, data: &Value) -> Result<String> {
        debug!("Rendering template: {}", template_name);
        match self.handlebars.render(template_name, data) {
            Ok(rendered) => Ok(rendered),
            Err(e) => {
                error!("Failed to render template {}: {}", template_name, e);
                Err(ToolkitError::TemplateError(format!(
                    "Failed to render template '{}': {}", 
                    template_name, e
                )))
            }
        }
    }
    
    /// Add a new template or update an existing one.
    ///
    /// This method registers a new template with the Handlebars engine and
    /// saves it to the template directory.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the template.
    /// * `content` - The template content.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be registered or saved.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    /// use std::path::Path;
    ///
    /// let mut manager = PromptManager::new(Path::new("./templates")).unwrap();
    /// manager.add_template("custom", "This is a {{project_name}} template.").unwrap();
    /// ```
    pub fn add_template(&mut self, name: &str, content: &str) -> Result<()> {
        debug!("Adding/updating template: {}", name);
        match self.handlebars.register_template_string(name, content) {
            Ok(_) => {
                // Save the template to disk
                let template_path = self.template_dir.join(format!("{}.hbs", name));
                fs::write(template_path, content)?;
                Ok(())
            },
            Err(e) => {
                error!("Failed to register template {}: {}", name, e);
                Err(ToolkitError::TemplateError(format!(
                    "Failed to register template '{}': {}", 
                    name, e
                )))
            }
        }
    }
    
    /// Check if a template exists.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the template to check.
    ///
    /// # Returns
    ///
    /// `true` if the template exists, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    /// use std::path::Path;
    ///
    /// let manager = PromptManager::new(Path::new("./templates")).unwrap();
    /// if manager.has_template("stage1") {
    ///     println!("Template exists!");
    /// }
    /// ```
    pub fn has_template(&self, name: &str) -> bool {
        self.handlebars.has_template(name)
    }
    
    /// Get all registered template names.
    ///
    /// # Returns
    ///
    /// A vector of template names.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    /// use std::path::Path;
    ///
    /// let manager = PromptManager::new(Path::new("./templates")).unwrap();
    /// let template_names = manager.get_template_names();
    /// for name in template_names {
    ///     println!("Template: {}", name);
    /// }
    /// ```
    pub fn get_template_names(&self) -> Vec<String> {
        self.handlebars.get_templates().keys().cloned().collect()
    }
    
    /// Convert a HashMap of variables into a serde_json::Value for template rendering.
    ///
    /// This utility method converts a simple string-to-string HashMap into a JSON
    /// value that can be used for template rendering.
    ///
    /// # Parameters
    ///
    /// * `vars` - A HashMap of variable names to values.
    ///
    /// # Returns
    ///
    /// A JSON value containing the variables.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::prompts::PromptManager;
    /// use std::collections::HashMap;
    ///
    /// let mut vars = HashMap::new();
    /// vars.insert("project_name".to_string(), "My Project".to_string());
    /// vars.insert("description".to_string(), "A sample project".to_string());
    ///
    /// let data = PromptManager::vars_to_json(vars);
    /// ```
    pub fn vars_to_json(vars: HashMap<String, String>) -> Value {
        serde_json::to_value(vars).unwrap_or_default()
    }
    
    /// Create a default global prompt manager.
    ///
    /// This method creates a prompt manager that uses a standard location
    /// in the user's home directory for storing templates.
    ///
    /// # Returns
    ///
    /// A `Result` containing the new `PromptManager` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the template directory cannot be created or if
    /// templates cannot be loaded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::prompts::PromptManager;
    ///
    /// let manager = PromptManager::global().unwrap();
    /// ```
    pub fn global() -> Result<Self> {
        let home_dir = dirs::home_dir().expect("Failed to find home directory");
        let templates_dir = home_dir.join(".rust-ai-toolkit").join("templates");
        Self::new(&templates_dir)
    }
}

/// Default templates for each stage of AI interaction.
///
/// This module contains predefined templates that are used as fallbacks
/// when a requested template is not found in the template directory.
pub mod templates {
    use lazy_static::lazy_static;
    use std::collections::HashMap;
    
    lazy_static! {
        /// Default templates for common AI interactions.
        ///
        /// These templates are used as fallbacks when a requested template
        /// is not found in the template directory. They cover various stages
        /// of project development and AI interaction.
        ///
        /// Available templates:
        /// - `stage1`: Initial Plan Creation
        /// - `stage2`: Architecture Design
        /// - `stage3`: Implementation Strategy
        /// - `stage4`: Progress Assessment
        /// - `stage5`: User Experience Design
        pub static ref DEFAULT_TEMPLATES: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            
            // Stage 1: Initial Plan Creation
            m.insert("stage1", r#"# Initial Plan Creation

I have a project idea that I'd like you to develop into a comprehensive plan.

## Project Idea
{{project_idea}}

## Task
Please take this rough idea and develop it into a comprehensive plan.
Include the following:

1. Technical details and architecture
2. Key features and user stories
3. Implementation approaches
4. Timeline and milestones
5. Potential challenges and solutions

Make the plan thorough and ambitious, capturing the full vision of what this project could be.
Format your response in Markdown with clear sections and structure.
"#);
            
            // Stage 2: Architecture Design
            m.insert("stage2", r#"# Architecture Design

## Project Background
{{project_description}}

## Initial Plan
{{initial_plan}}

## Task
Based on the initial plan, create a comprehensive software architecture design:

1. System components and their interactions
2. Data models and storage strategies
3. API design (if applicable)
4. Technology stack recommendations with justifications
5. Diagrams or visual representations (describe in text)
6. Performance, security, and scalability considerations

Provide extensive detail on each component and how they work together.
Format your response in Markdown with clear sections and structure.
"#);
            
            // Stage 3: Implementation Strategy
            m.insert("stage3", r#"# Implementation Strategy

## Project Overview
{{project_description}}

## Architecture Design
{{architecture_design}}

## Task
Develop a detailed implementation strategy that includes:

1. Development roadmap with phases
2. Core functionality implementation details
3. Critical path analysis
4. Resource requirements
5. Testing strategy and quality assurance approach
6. Deployment considerations

Break down complex components into manageable tasks and explain the approach for implementing each one.
Format your response in Markdown with clear sections and structure.
"#);

            // Stage 4: Progress Assessment
            m.insert("stage4", r#"# Progress Assessment

## Project Overview
{{project_description}}

## Implementation Strategy
{{implementation_strategy}}

## Current Status
{{current_status}}

## Task
Provide a comprehensive progress assessment:

1. Evaluate what has been accomplished so far
2. Identify any bottlenecks or challenges faced
3. Suggest adjustments to the original plan if needed
4. Recommend next steps with prioritization
5. Provide technical guidance for overcoming any obstacles

Be honest and constructive in your assessment. Focus on actionable advice.
Format your response in Markdown with clear sections and structure.
"#);

            // Stage 5: User Experience Design
            m.insert("stage5", r#"# User Experience Design

## Project Overview
{{project_description}}

## Architecture Design
{{architecture_design}}

## Task
Create a user experience design strategy that includes:

1. User personas and journey maps
2. Interface design principles and guidelines
3. Wireframes or mockups (describe in text)
4. Interaction patterns and navigation flow
5. Accessibility considerations
6. User testing approach

Focus on creating an intuitive, engaging, and accessible user experience.
Format your response in Markdown with clear sections and structure.
"#);

            m
        };
    }
}

// Re-export key items for easier access
pub use templates::DEFAULT_TEMPLATES; 