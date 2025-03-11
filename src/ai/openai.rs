use crate::error::{Result, ToolkitError};
use crate::utils::rate_limiter;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use log::{debug, error, warn};
use crate::config;
use std::time::Duration;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use serde_json::Value;
use super::{RequestOptions, FunctionDefinition, SHARED_HTTP_CLIENT, headers};

pub struct OpenAiClient {
    api_key: String,
    model: String,
    base_url: String,
    api_version: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Vec<FunctionDefinition>>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

// Streaming responses
#[derive(Deserialize, Debug)]
struct StreamingResponse {
    choices: Vec<StreamingChoice>,
}

#[derive(Deserialize, Debug)]
struct StreamingChoice {
    delta: StreamingDelta,
}

#[derive(Deserialize, Debug)]
struct StreamingDelta {
    #[serde(default)]
    content: String,
}

impl OpenAiClient {
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            error!("OpenAI API key is not configured");
            return Err(ToolkitError::Config(
                "OpenAI API key is not configured. Please run 'rust-ai-toolkit config' to set up your API key.".to_string(),
            ));
        }
        
        let config = config::get_config()?;
        let base_url = config.base_url.clone().unwrap_or_else(|| 
            "https://api.openai.com/v1".to_string()
        );
        
        // Use a hardcoded API version since it's not in the Config struct
        let api_version = "2024-02-15".to_string();

        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url,
            api_version,
        })
    }
    
    fn create_request_body(&self, prompt: &str, options: &RequestOptions, stream: bool) -> OpenAiRequest {
        OpenAiRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            stream: Some(stream),
            functions: options.functions.clone(),
        }
    }

    async fn send_request(
        &self,
        request: OpenAiRequest,
        streaming: bool,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        // Use the shared HTTP client instead of creating a new one
        let client = &*SHARED_HTTP_CLIENT;
        
        // Check rate limits
        if !rate_limiter::can_make_request("openai") {
            return Err(ToolkitError::RateLimit(
                "OpenAI API rate limit exceeded. Please try again later.".to_string(),
            ));
        }
        
        // Record this request
        rate_limiter::record_request("openai");
        
        let url = format!("{}/chat/completions", self.base_url);
        
        let mut builder = client.post(&url)
            .header(headers::AUTHORIZATION, format!("{}{}", headers::BEARER_PREFIX, &self.api_key))
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
                error!("Failed to send request to OpenAI API: {}", e);
                // Record failure for rate limiting
                let backoff = rate_limiter::record_failure("openai");
                ToolkitError::Api(format!("Failed to send request to OpenAI API: {}. Backing off for {}ms", e, backoff))
            })?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI API error: {} - {}", status, error_text);
            
            // Check if this is a rate limit error
            if status.as_u16() == 429 {
                // Record a rate limit failure for a longer backoff
                rate_limiter::record_rate_limit("openai");
                return Err(ToolkitError::RateLimit(
                    "OpenAI API rate limit exceeded. Please wait before making more requests.".to_string()
                ));
            }
            
            return Err(ToolkitError::Api(
                format!("OpenAI API error: {} - {}", status, error_text)
            ));
        }
        
        Ok(response)
    }
}

#[async_trait]
impl super::AiClient for OpenAiClient {
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
        
        if !rate_limiter::can_make_request("openai") {
            warn!("Rate limit exceeded for OpenAI API");
            return Err(ToolkitError::Api(
                "Rate limit exceeded for OpenAI API. Please try again later.".to_string(),
            ));
        }
        
        rate_limiter::record_request("openai");
        
        let request = self.create_request_body(prompt, &options, false);
        let response = self.send_request(request, false, options.timeout).await?;
        
        let response_data: OpenAiResponse = response.json().await.map_err(|e| {
            error!("Failed to parse OpenAI API response: {}", e);
            ToolkitError::Parse(e.to_string())
        })?;
        
        if response_data.choices.is_empty() {
            return Err(ToolkitError::Api("No response from OpenAI API".to_string()));
        }
        
        Ok(response_data.choices[0].message.content.clone())
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
        
        if !rate_limiter::can_make_request("openai") {
            warn!("Rate limit exceeded for OpenAI API");
            return Err(ToolkitError::Api(
                "Rate limit exceeded for OpenAI API. Please try again later.".to_string(),
            ));
        }
        
        rate_limiter::record_request("openai");
        
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
                    
                    if response.choices.is_empty() {
                        return Ok("".to_string());
                    }
                    
                    Ok(response.choices[0].delta.content.clone())
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
