# AI Provider Configuration

## Overview

The Rust AI Toolkit supports multiple AI providers, allowing you to choose the service that best fits your needs. This document covers the supported providers, their configuration options, and best practices for each.

## Supported Providers

The toolkit currently supports the following AI providers:

1. **OpenAI** - Provider of GPT models
2. **Anthropic** - Provider of Claude models
3. **Anthropic Enhanced** - Extended version of Anthropic's API with additional features

Each provider has different capabilities, pricing structures, and rate limits. Choose the one that best aligns with your requirements and budget.

## Configuration

### Initial Setup

Configure your AI provider using the interactive configuration command:

```bash
rust-ai-toolkit config
```

This will guide you through setting up your preferred provider, API key, and other options.

### Manual Configuration

You can also edit the configuration file directly. The configuration is stored in:

```
~/.rust-ai-toolkit/config.toml
```

Example configuration:

```toml
[ai]
provider = "anthropic"
api_key = "your-api-key-here"
model = "claude-3-opus-20240229"
base_url = "https://api.anthropic.com/v1"

[rate_limit]
requests_per_minute = 30
warn_threshold = 0.8
```

## Provider-Specific Configuration

### OpenAI

OpenAI provides GPT models with varying capabilities and costs.

#### Configuration Options

```toml
[ai]
provider = "openai"
api_key = "your-openai-api-key"
model = "gpt-4"  # or "gpt-3.5-turbo", etc.
base_url = "https://api.openai.com/v1"  # Optional, defaults to OpenAI's API
```

#### Recommended Models

- **gpt-4** - Most capable model, best for complex reasoning
- **gpt-3.5-turbo** - Faster and more cost-effective for simpler tasks

#### Rate Limits

OpenAI imposes rate limits based on your account tier:

- Free tier: ~3 RPM (requests per minute)
- Pay-as-you-go: ~60 RPM for GPT-3.5, ~40 RPM for GPT-4

Configure your rate limits accordingly:

```toml
[rate_limit]
requests_per_minute = 60  # Adjust based on your tier
```

### Anthropic

Anthropic provides Claude models known for their helpfulness and harmlessness.

#### Configuration Options

```toml
[ai]
provider = "anthropic"
api_key = "your-anthropic-api-key"
model = "claude-3-opus-20240229"  # or other Claude models
base_url = "https://api.anthropic.com/v1"
```

#### Recommended Models

- **claude-3-opus** - Most capable model, best for complex tasks
- **claude-3-sonnet** - Good balance of capability and cost
- **claude-3-haiku** - Fastest and most cost-effective

#### Rate Limits

Anthropic typically allows ~30 RPM for standard accounts:

```toml
[rate_limit]
requests_per_minute = 30
```

### Anthropic Enhanced

This is an extended implementation of the Anthropic API with additional features specific to the Rust AI Toolkit.

#### Configuration Options

```toml
[ai]
provider = "anthropic_enhanced"
api_key = "your-anthropic-api-key"
model = "claude-3-opus-20240229"
base_url = "https://api.anthropic.com/v1"
```

#### Additional Features

- Improved error handling
- Better streaming support
- Enhanced function calling capabilities

## Advanced Configuration

### Timeout Settings

Set custom timeout values for API requests:

```toml
[ai]
request_timeout_seconds = 120  # Default is 60
```

### Proxy Configuration

If you need to use a proxy:

```toml
[network]
proxy_url = "http://your-proxy-server:port"
```

### Custom Base URLs

For enterprise deployments or custom endpoints:

```toml
[ai]
base_url = "https://your-custom-endpoint.com/v1"
```

## Function Calling

The toolkit supports function calling capabilities with compatible models. This allows the AI to request specific actions or data during generation.

### Supported Providers for Function Calling

- **OpenAI**: Full support with GPT-4 and newer GPT-3.5 models
- **Anthropic Enhanced**: Experimental support with Claude 3 models

### Example Function Definition

```rust
let function = FunctionDefinition {
    name: "get_weather".to_string(),
    description: "Get the current weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "The city and state, e.g., San Francisco, CA"
            }
        },
        "required": ["location"]
    }),
};
```

## Performance Considerations

### Model Selection

- Larger models (GPT-4, Claude 3 Opus) provide better quality but are slower and more expensive
- Smaller models (GPT-3.5-Turbo, Claude 3 Haiku) are faster and cheaper but may produce lower quality results

### Caching

The toolkit implements caching to reduce API calls:

```toml
[cache]
enabled = true
ttl_hours = 24  # How long to keep cached responses
```

### Rate Limiting

Configure rate limiting to avoid hitting provider limits:

```toml
[rate_limit]
requests_per_minute = 30
warn_threshold = 0.8  # Warn when reaching 80% of the limit
backoff_factor = 2.0  # Exponential backoff factor for retries
```

## Troubleshooting

### API Key Issues

If you encounter authentication errors:
1. Verify your API key is correct
2. Check that your account has access to the requested model
3. Ensure your account has sufficient credits/quota

### Rate Limit Errors

If you hit rate limits:
1. Reduce the `requests_per_minute` setting
2. Implement longer delays between requests
3. Consider upgrading your API provider account

### Timeout Errors

For timeout issues:
1. Increase the `request_timeout_seconds` setting
2. Check your network connection
3. Try a different model that responds faster

## Provider-Specific Error Codes

### OpenAI

- `401`: Invalid API key
- `429`: Rate limit exceeded
- `500`: Server error

### Anthropic

- `401`: Invalid API key
- `429`: Rate limit exceeded
- `500`: Server error

## Best Practices

1. **Start with smaller models** for development and testing
2. **Enable caching** to reduce API costs during development
3. **Set conservative rate limits** to avoid service disruptions
4. **Store API keys securely** and never commit them to version control
5. **Monitor usage** to control costs

## Switching Providers

You can switch providers at any time:

```bash
rust-ai-toolkit config --provider anthropic --api-key your-new-key
```

Your project data will remain intact, though results may vary between providers. 