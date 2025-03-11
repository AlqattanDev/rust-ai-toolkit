# Rust AI Toolkit

A command-line toolkit for automating a staged approach to project planning and development with AI.

## Features

- Multi-stage project planning process
- Integration with various AI providers
- Customizable prompt templates
- Extensible stage-based architecture

## Installation

```bash
cargo install rust-ai-toolkit
```

## Usage

```bash
# Initialize a new project
rust-ai-toolkit init -n "Project Name" -d "Project description"

# Run a specific stage
rust-ai-toolkit run-stage -s 1 -p project_id

# List all projects
rust-ai-toolkit list

# Show project status
rust-ai-toolkit status -p project_id

# Configure AI provider
rust-ai-toolkit config
```

For detailed usage instructions, see the [Usage Guide](docs/USAGE.md).

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- [Usage Guide](docs/USAGE.md) - Detailed usage instructions with examples
- [AI Provider Configuration](docs/API_PROVIDERS.md) - Information about supported AI providers
- [Template Customization](docs/TEMPLATES.md) - Guide to using and customizing prompt templates
- [Troubleshooting Guide](docs/TROUBLESHOOTING.md) - Common issues and solutions

## Examples

The `examples/` directory contains runnable code examples demonstrating various features of the toolkit:

- [Basic Usage](examples/basic_usage.rs) - Simple initialization and running a stage
- [Custom Templates](examples/custom_template.rs) - Creating and using custom prompt templates
- [Streaming Responses](examples/streaming_responses.rs) - Working with streaming AI responses
- [Rate Limit Handling](examples/rate_limit_handling.rs) - Proper handling of rate limits and backoff
- [Custom AI Client](examples/custom_ai_client.rs) - Implementing a custom AI provider

See the [Examples README](examples/README.md) for instructions on running these examples.

## Architecture

The toolkit is organized around a modular architecture:

### Core Components

1. **Prompt System**: Customizable templates for AI interactions
   - Templates are stored in Handlebars format in `~/.rust-ai-toolkit/templates/`
   - Default templates are provided but can be customized

2. **Stage System**: Trait-based approach for project development stages
   - Each stage implements the `Stage` trait
   - Stages can declare dependencies on other stages
   - Stages share context data through a context object

3. **Utility Modules**: Common functionality for all components
   - File operations
   - User interaction
   - Project management

### Project Structure

```
src/
├── ai/          # AI provider integrations
├── config/      # Configuration management
├── error/       # Error handling
├── models/      # Data models
├── prompts/     # Prompt templating system
├── stages/      # Project development stages
└── utils/       # Utility functions
```

## Customizing Prompts

You can customize the prompts used by the toolkit by editing the template files in `~/.rust-ai-toolkit/templates/`. The templates use Handlebars syntax with variables specific to each stage.

For detailed information on customizing templates, see the [Template Customization Guide](docs/TEMPLATES.md).

## Troubleshooting

If you encounter issues while using the toolkit, please refer to our [Troubleshooting Guide](docs/TROUBLESHOOTING.md) for common problems and solutions.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
