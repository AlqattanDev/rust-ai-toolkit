# Rust AI Toolkit Examples

This directory contains example code demonstrating various features and use cases of the Rust AI Toolkit.

## Setup

Before running any examples, make sure you have:

1. Set up your environment variables in a `.env` file in the project root:
   ```
   OPENAI_API_KEY=your_openai_api_key
   ANTHROPIC_API_KEY=your_anthropic_api_key
   # Add other provider keys as needed
   ```

2. Installed the Rust AI Toolkit dependencies:
   ```
   cargo build
   ```

## Running Examples

To run an example, use the following command:

```
cargo run --example <example_name>
```

For instance:

```
cargo run --example basic_usage
```

## Available Examples

### 1. Basic Usage (`basic_usage.rs`)

Demonstrates the simplest way to initialize the toolkit and run a stage. This is a good starting point for new users.

### 2. Custom Templates (`custom_template.rs`)

Shows how to create and use custom prompt templates with the toolkit, allowing you to tailor the AI's responses to your specific needs.

### 3. Streaming Responses (`streaming_responses.rs`)

Illustrates how to work with streaming AI responses, processing content as it arrives rather than waiting for the complete response.

### 4. Rate Limit Handling (`rate_limit_handling.rs`)

Demonstrates proper handling of rate limits and backoff strategies when working with AI providers that impose request limits.

### 5. Custom AI Client (`custom_ai_client.rs`)

Shows how to implement a custom AI provider by creating a client that implements the `AiClient` trait, allowing you to integrate with any AI service.

## Troubleshooting

If you encounter issues running the examples:

1. Ensure your API keys are correctly set in the `.env` file
2. Check that you have the latest version of the Rust AI Toolkit
3. Verify that all dependencies are installed with `cargo build`
4. For provider-specific issues, refer to the [API_PROVIDERS.md](../docs/API_PROVIDERS.md) documentation

## Additional Resources

For more detailed information about the Rust AI Toolkit, refer to the documentation in the `docs/` directory:

- [Usage Guide](../docs/USAGE.md)
- [API Providers](../docs/API_PROVIDERS.md)
- [Templates](../docs/TEMPLATES.md)
- [Troubleshooting](../docs/TROUBLESHOOTING.md)