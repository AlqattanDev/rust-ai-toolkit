# Rust AI Toolkit: Usage Guide

## Overview

The Rust AI Toolkit is a command-line application designed to streamline project planning and development using AI assistance. This guide provides detailed instructions on how to use the toolkit effectively, from initial setup to advanced features.

## Getting Started

### Installation

Install the Rust AI Toolkit using Cargo:

```bash
cargo install rust-ai-toolkit
```

### Initial Configuration

Before using the toolkit, you need to configure your AI provider:

```bash
rust-ai-toolkit config
```

This interactive command will prompt you for:
- AI provider (OpenAI, Anthropic, etc.)
- API key
- Default model to use
- Rate limiting preferences

Your configuration is stored in `~/.rust-ai-toolkit/config.toml` and can be edited manually if needed.

## Core Commands

### Creating a New Project

Initialize a new project with:

```bash
rust-ai-toolkit init -n "My Project Name" -d "A detailed description of my project"
```

This command:
1. Creates a new project entry in the toolkit's database
2. Assigns a unique project ID
3. Prepares the project for the first stage

Optional flags:
- `-p, --path <PATH>`: Specify a directory for project files (defaults to current directory)
- `-t, --tags <TAGS>`: Add comma-separated tags to categorize your project

### Running Project Stages

The toolkit uses a staged approach to project development. Run a specific stage with:

```bash
rust-ai-toolkit run-stage -s <STAGE_NUMBER> -p <PROJECT_ID>
```

For example:
```bash
rust-ai-toolkit run-stage -s 1 -p proj_12345
```

Available stages:
1. Initial Plan Creation
2. Architecture Design
3. Implementation Strategy
4. Progress Assessment
5. User Experience Design

Each stage builds upon the previous ones, so it's recommended to run them in sequence.

### Managing Projects

List all your projects:

```bash
rust-ai-toolkit list
```

View detailed information about a specific project:

```bash
rust-ai-toolkit status -p <PROJECT_ID>
```

Export a project's outputs:

```bash
rust-ai-toolkit export -p <PROJECT_ID> -o <OUTPUT_DIRECTORY>
```

Delete a project:

```bash
rust-ai-toolkit delete -p <PROJECT_ID>
```

## Advanced Usage

### Custom Prompt Variables

You can provide custom variables to the prompts when running a stage:

```bash
rust-ai-toolkit run-stage -s 1 -p proj_12345 -v "key1=value1,key2=value2"
```

These variables will be available in the templates as `{{key1}}` and `{{key2}}`.

### Interactive Mode

Run a stage in interactive mode to have a conversation with the AI:

```bash
rust-ai-toolkit run-stage -s 1 -p proj_12345 --interactive
```

In this mode, you can:
- Provide additional context during the stage
- Ask follow-up questions
- Refine the AI's responses

### Batch Processing

Process multiple projects or stages at once:

```bash
rust-ai-toolkit batch -f batch_file.json
```

Where `batch_file.json` contains an array of operations to perform:

```json
[
  {
    "operation": "run-stage",
    "project_id": "proj_12345",
    "stage": 1
  },
  {
    "operation": "run-stage",
    "project_id": "proj_67890",
    "stage": 2
  }
]
```

### Using Project Output in Subsequent Stages

Each stage's output is automatically available to later stages through template variables:

- Stage 1 output is available as `{{initial_plan}}`
- Stage 2 output is available as `{{architecture_design}}`
- Stage 3 output is available as `{{implementation_strategy}}`
- Stage 4 output is available as `{{progress_assessment}}`

You can reference these in custom templates or when providing additional context.

## Working with Project Files

### Generating Code

Some stages can generate code snippets. Save these to files with:

```bash
rust-ai-toolkit extract-code -p <PROJECT_ID> -s <STAGE_NUMBER> -o <OUTPUT_DIRECTORY>
```

### Importing Existing Code

Import existing code to provide context to the AI:

```bash
rust-ai-toolkit import-code -p <PROJECT_ID> -f <FILE_PATH> -d "Description of this code"
```

This code will be available to reference in future stages.

## Performance Optimization

### Caching

The toolkit caches AI responses to improve performance and reduce API costs. Control caching behavior with:

```bash
# Clear cache for a specific project
rust-ai-toolkit clear-cache -p <PROJECT_ID>

# Clear all caches
rust-ai-toolkit clear-cache --all

# Set cache expiration time (in hours)
rust-ai-toolkit config --cache-ttl 24
```

### Rate Limiting

Configure rate limiting to avoid hitting API provider limits:

```bash
rust-ai-toolkit config --rate-limit 30  # 30 requests per minute
```

## Examples

### Complete Project Workflow

```bash
# Initialize a new web application project
rust-ai-toolkit init -n "E-commerce Platform" -d "A modern e-commerce platform with user accounts, product catalog, and payment processing"

# The command outputs a project ID, e.g., proj_12345

# Run the initial planning stage
rust-ai-toolkit run-stage -s 1 -p proj_12345

# Review the initial plan
rust-ai-toolkit status -p proj_12345

# Run the architecture design stage
rust-ai-toolkit run-stage -s 2 -p proj_12345

# Run the implementation strategy stage
rust-ai-toolkit run-stage -s 3 -p proj_12345

# Export all project outputs to a directory
rust-ai-toolkit export -p proj_12345 -o ./my-ecommerce-project
```

### Customizing a Stage with Additional Context

```bash
# Run a stage with custom variables
rust-ai-toolkit run-stage -s 4 -p proj_12345 -v "current_status=Backend API is complete but frontend needs work,priority=Completing the user authentication flow"
```

## Troubleshooting

If you encounter issues, check the [Troubleshooting Guide](TROUBLESHOOTING.md) for common problems and solutions.

For more specific topics, refer to:
- [AI Provider Configuration](API_PROVIDERS.md)
- [Template Customization](TEMPLATES.md) 