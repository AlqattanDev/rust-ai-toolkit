use crate::error::{Result, ToolkitError};
use crate::utils::rate_limiter;
use async_trait::async_trait;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::{debug, error, info, warn};
use crate::config;
use std::time::Duration;

pub struct EnhancedAnthropicClient {
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    tools: Vec<Tool>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Serialize)]
struct Tool {
    #[serde(rename = "type")]
    tool_type: String,
    function: Function,
}

#[derive(Serialize)]
struct Function {
    name: String,
    description: String,
    parameters: Parameters,
}

#[derive(Serialize)]
struct Parameters {
    #[serde(rename = "type")]
    param_type: String,
    properties: HashMap<String, PropertyDetails>,
    required: Vec<String>,
}

#[derive(Serialize)]
struct PropertyDetails {
    #[serde(rename = "type")]
    prop_type: String,
    description: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

impl EnhancedAnthropicClient {
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            error!("Anthropic API key is not configured");
            return Err(ToolkitError::Config(
                "Anthropic API key is not configured. Please run 'rust-ai-toolkit config' to set up your API key.".to_string(),
            ));
        }
        
        if !api_key.starts_with("sk-ant-") {
            println!("{}", "Warning: Your Anthropic API key should typically start with 'sk-ant-'.".yellow());
            println!("{}", "If you're having authentication issues, please check your API key.".yellow());
        }
        
        // Log masked API key for security
        debug!("Creating Enhanced Anthropic client with API key: {} and model: {}", 
            config::mask_api_key(api_key), model);
        
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }
    
    fn create_code_tools() -> Vec<Tool> {
        let mut properties = HashMap::new();
        properties.insert(
            "language".to_string(),
            PropertyDetails {
                prop_type: "string".to_string(),
                description: "The programming language of the code".to_string(),
            },
        );
        properties.insert(
            "code".to_string(),
            PropertyDetails {
                prop_type: "string".to_string(),
                description: "The code to analyze".to_string(),
            },
        );
        
        vec![
            Tool {
                tool_type: "function".to_string(),
                function: Function {
                    name: "analyze_code".to_string(),
                    description: "Analyze code for improvements, bugs, and optimization opportunities".to_string(),
                    parameters: Parameters {
                        param_type: "object".to_string(),
                        properties,
                        required: vec!["language".to_string(), "code".to_string()],
                    },
                },
            },
        ]
    }
}

#[async_trait]
impl super::AiClient for EnhancedAnthropicClient {
    fn model_version(&self) -> &str {
        &self.model
    }

    fn base_url(&self) -> &str {
        "https://api.anthropic.com/v1"
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        debug!("Generating response with model: {}", self.model);
        debug!("Prompt length: {} characters", prompt.len());
        
        // Check rate limit before making request
        if !rate_limiter::can_make_request("anthropic_enhanced") {
            warn!("Rate limit exceeded for Anthropic API");
            return Err(ToolkitError::Api(
                "Rate limit exceeded for Anthropic API. Please try again later.".to_string(),
            ));
        }
        
        // Record this request
        rate_limiter::record_request("anthropic_enhanced");
        
        let client = reqwest::Client::new();
        
        // Create tools for code generation capabilities
        let tools = Self::create_code_tools();
        
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 4000,
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![Content {
                    content_type: "text".to_string(),
                    text: prompt.to_string(),
                }],
            }],
            tools,
        };
        
        info!("Sending request to Anthropic Enhanced API...");
        
        // Make the API request with retry logic
        let mut retry_count = 0;
        let max_retries = 3;
        
        loop {
            match client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        // Record successful request
                        rate_limiter::record_success("anthropic_enhanced");
                        
                        let response_body = response.json::<AnthropicResponse>().await.map_err(|e| {
                            error!("Failed to parse Anthropic Enhanced API response: {}", e);
                            ToolkitError::Api(format!("Failed to parse API response: {}", e))
                        })?;
                        
                        // Extract text from the response
                        let text = response_body
                            .content
                            .iter()
                            .filter(|c| c.content_type == "text")
                            .map(|c| c.text.clone())
                            .collect::<Vec<String>>()
                            .join("");
                        
                        info!("Received successful response from Anthropic Enhanced API");
                        debug!("Response length: {} characters", text.len());
                        
                        return Ok(text);
                    } else {
                        // Record failure
                        let backoff_ms = rate_limiter::record_failure("anthropic_enhanced");
                        
                        // Clone the status before consuming the response
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        error!("Anthropic Enhanced API error: {} - {}", status, error_text);
                        
                        if retry_count < max_retries {
                            retry_count += 1;
                            warn!("Retrying request ({}/{}), backing off for {}ms", 
                                retry_count, max_retries, backoff_ms);
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            continue;
                        }
                        
                        return Err(ToolkitError::Api(format!(
                            "API error: {} - {}",
                            status,
                            error_text
                        )));
                    }
                }
                Err(e) => {
                    // Record failure
                    let backoff_ms = rate_limiter::record_failure("anthropic_enhanced");
                    
                    error!("Anthropic Enhanced API request error: {}", e);
                    
                    if retry_count < max_retries {
                        retry_count += 1;
                        warn!("Retrying request ({}/{}), backing off for {}ms", 
                            retry_count, max_retries, backoff_ms);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    }
                    
                    return Err(ToolkitError::Network(e.to_string()));
                }
            }
        }
    }
}
