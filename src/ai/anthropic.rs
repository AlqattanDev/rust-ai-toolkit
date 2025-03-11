use crate::error::{Result, ToolkitError};
use crate::utils::rate_limiter;
use crate::utils::logging;
use crate::config;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;
use super::{RequestOptions, FunctionDefinition, SHARED_HTTP_CLIENT, headers};

// Define constants for hardcoded values
/// The default Anthropic API version
pub const ANTHROPIC_API_VERSION: &str = "2024-02-15";
/// Default base URL for Anthropic API
pub const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
/// Expected prefix for Anthropic API keys
pub const ANTHROPIC_API_KEY_PREFIX: &str = "sk-ant-";
/// Content type for text
pub const CONTENT_TYPE_TEXT: &str = "text";
/// Role for user messages
pub const ROLE_USER: &str = "user";

pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    api_version: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: Option<u32>,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<FunctionDefinition>>,
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

#[derive(Deserialize)]
struct StreamingResponse {
    delta: StreamingDelta,
}

#[derive(Deserialize)]
struct StreamingDelta {
    #[serde(default)]
    text: String,
}

impl AnthropicClient {
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            error!("Anthropic API key is not configured");
            return Err(ToolkitError::Config(
                "Anthropic API key is not configured. Please run 'rust-ai-toolkit config' to set up your API key.".to_string(),
            ));
        }
        
        if !api_key.starts_with(ANTHROPIC_API_KEY_PREFIX) {
            warn!("Anthropic API key format warning: key doesn't start with expected prefix '{}'", ANTHROPIC_API_KEY_PREFIX);
            logging::warn_user(&format!("Warning: Your Anthropic API key should typically start with '{}'.", ANTHROPIC_API_KEY_PREFIX));
            logging::warn_user("If you're having authentication issues, please check your API key.");
        }
        
        let config = config::get_config()?;
        let base_url = config.base_url.unwrap_or_else(|| 
            ANTHROPIC_BASE_URL.to_string()
        );
        
        let api_version = ANTHROPIC_API_VERSION.to_string();

        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url,
            api_version,
        })
    }
    
    fn create_request_body(&self, prompt: &str, options: &RequestOptions, stream: bool) -> AnthropicRequest {
        let content = Content {
            content_type: CONTENT_TYPE_TEXT.to_string(),
            text: prompt.to_string(),
        };
        
        let message = Message {
            role: ROLE_USER.to_string(),
            content: vec![content],
        };
        
        // Use references instead of cloning when possible
        AnthropicRequest {
            model: self.model.clone(),
            max_tokens: options.max_tokens,
            messages: vec![message],
            temperature: options.temperature,
            top_p: options.top_p,
            stream: Some(stream),
            tools: options.functions.clone(),
        }
    }

    async fn send_request(
        &self,
        request: AnthropicRequest,
        streaming: bool,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        // Use the shared HTTP client
        let client = &*SHARED_HTTP_CLIENT;
        
        // Check rate limits
        if !rate_limiter::can_make_request("anthropic") {
            return Err(ToolkitError::RateLimit(
                "Anthropic API rate limit exceeded. Please try again later.".to_string(),
            ));
        }
        
        // Record this request
        rate_limiter::record_request("anthropic");
        
        let url = format!("{}/messages", self.base_url);
        
        let mut builder = client.post(&url)
            .header(headers::X_API_KEY, &self.api_key)
            .header(headers::ANTHROPIC_VERSION, &self.api_version)
            .header(headers::CONTENT_TYPE, headers::APPLICATION_JSON);
        
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }
        
        if streaming {
            builder = builder.header(headers::ACCEPT, headers::TEXT_EVENT_STREAM);
        }
        
        let response = builder
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to Anthropic API: {}", e);
                // Record failure for rate limiting
                let backoff = rate_limiter::record_failure("anthropic");
                ToolkitError::Api(format!("Failed to send request to Anthropic API: {}. Backing off for {}ms", e, backoff))
            })?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Anthropic API error: {} - {}", status, error_text);
            
            // Check if this is a rate limit error
            if status.as_u16() == 429 {
                // Record a rate limit failure for a longer backoff
                rate_limiter::record_rate_limit("anthropic");
                return Err(ToolkitError::RateLimit(
                    "Anthropic API rate limit exceeded. Please wait before making more requests.".to_string()
                ));
            }
            
            return Err(ToolkitError::Api(
                format!("Anthropic API error: {} - {}", status, error_text)
            ));
        }
        
        Ok(response)
    }
}

#[async_trait]
impl super::AiClient for AnthropicClient {
    fn model_version(&self) -> &str {
        &self.api_version
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        let options = RequestOptions::default();
        self.generate_with_options(prompt, options).await
    }

    async fn generate_with_options(&self, prompt: &str, options: RequestOptions) -> Result<String> {
        debug!("Generating response with model: {}", self.model);
        
        if !rate_limiter::can_make_request("anthropic") {
            warn!("Rate limit exceeded for Anthropic API");
            return Err(ToolkitError::Api(
                "Rate limit exceeded for Anthropic API. Please try again later.".to_string(),
            ));
        }
        
        rate_limiter::record_request("anthropic");
        
        let request = self.create_request_body(prompt, &options, false);
        let response = self.send_request(request, false, options.timeout).await?;
        
        let response_data: AnthropicResponse = response.json().await.map_err(|e| {
            error!("Failed to parse Anthropic API response: {}", e);
            ToolkitError::Parse(e.to_string())
        })?;
        
        if response_data.content.is_empty() {
            return Err(ToolkitError::Api("No response from Anthropic API".to_string()));
        }
        
        Ok(response_data.content[0].text.clone())
    }

    async fn generate_streaming(&self, prompt: &str) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let options = RequestOptions::default();
        self.generate_streaming_with_options(prompt, options).await
    }

    async fn generate_streaming_with_options(
        &self,
        prompt: &str,
        options: RequestOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        debug!("Generating streaming response with model: {}", self.model);
        
        if !rate_limiter::can_make_request("anthropic") {
            warn!("Rate limit exceeded for Anthropic API");
            return Err(ToolkitError::Api(
                "Rate limit exceeded for Anthropic API. Please try again later.".to_string(),
            ));
        }
        
        rate_limiter::record_request("anthropic");
        
        let request = self.create_request_body(prompt, &options, true);
        let response = self.send_request(request, true, options.timeout).await?;
        
        let stream = response.bytes_stream().map(|result| {
            result.map_err(|e| ToolkitError::Network(e.to_string()))
                .and_then(|bytes| {
                    let text = String::from_utf8(bytes.to_vec())
                        .map_err(|e| ToolkitError::Parse(e.to_string()))?;
                    
                    if text.trim().is_empty() {
                        return Ok("".to_string());
                    }
                    
                    let response: StreamingResponse = serde_json::from_str(&text)
                        .map_err(|e| ToolkitError::Parse(e.to_string()))?;
                    
                    Ok(response.delta.text.clone())
                })
        });
        
        Ok(Box::pin(stream))
    }

    async fn generate_json(&self, prompt: &str) -> Result<Value> {
        let options = RequestOptions::default();
        self.generate_json_with_options(prompt, options).await
    }

    async fn generate_json_with_options(&self, prompt: &str, options: RequestOptions) -> Result<Value> {
        let text = self.generate_with_options(prompt, options).await?;
        serde_json::from_str(&text).map_err(|e| ToolkitError::Parse(e.to_string()))
    }
}
