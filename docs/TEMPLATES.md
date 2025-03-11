# Template Customization Guide

## Overview

The Rust AI Toolkit uses a flexible templating system to generate prompts for AI interactions. This guide explains how to use the built-in templates, customize them for your needs, and create entirely new templates.

## Template Basics

Templates in the Rust AI Toolkit:
- Are written in Handlebars syntax
- Support variable substitution, conditionals, and loops
- Are stored as `.hbs` files in `~/.rust-ai-toolkit/templates/`
- Have default versions built into the toolkit

## Default Templates

The toolkit includes default templates for each stage of the project development process:

1. **Stage 1: Initial Plan Creation** (`stage1.hbs`)
2. **Stage 2: Architecture Design** (`stage2.hbs`)
3. **Stage 3: Implementation Strategy** (`stage3.hbs`)
4. **Stage 4: Progress Assessment** (`stage4.hbs`)
5. **Stage 5: User Experience Design** (`stage5.hbs`)

These templates are used automatically when running the corresponding stages.

## Template Location

Templates are stored in:

```
~/.rust-ai-toolkit/templates/
```

When you first run the toolkit, this directory is created and populated with the default templates.

## Viewing Templates

To view all available templates:

```bash
rust-ai-toolkit list-templates
```

To view the content of a specific template:

```bash
rust-ai-toolkit show-template -n stage1
```

## Template Syntax

Templates use Handlebars syntax:

### Variable Substitution

```handlebars
Hello, {{name}}!
```

When rendered with `{"name": "World"}`, produces: `Hello, World!`

### Conditionals

```handlebars
{{#if condition}}
  This will be rendered if condition is true.
{{else}}
  This will be rendered if condition is false.
{{/if}}
```

### Loops

```handlebars
{{#each items}}
  - {{this}}
{{/each}}
```

When rendered with `{"items": ["apple", "banana", "cherry"]}`, produces:
```
  - apple
  - banana
  - cherry
```

## Available Variables

Each template has access to specific variables depending on the stage:

### Stage 1: Initial Plan Creation

- `project_name`: The name of the project
- `project_description`: The description of the project
- `project_idea`: Alias for project_description

### Stage 2: Architecture Design

- `project_name`: The name of the project
- `project_description`: The description of the project
- `initial_plan`: The output from Stage 1

### Stage 3: Implementation Strategy

- `project_name`: The name of the project
- `project_description`: The description of the project
- `initial_plan`: The output from Stage 1
- `architecture_design`: The output from Stage 2

### Stage 4: Progress Assessment

- `project_name`: The name of the project
- `project_description`: The description of the project
- `initial_plan`: The output from Stage 1
- `architecture_design`: The output from Stage 2
- `implementation_strategy`: The output from Stage 3
- `current_status`: Current project status (if provided)

### Stage 5: User Experience Design

- `project_name`: The name of the project
- `project_description`: The description of the project
- `initial_plan`: The output from Stage 1
- `architecture_design`: The output from Stage 2
- `implementation_strategy`: The output from Stage 3
- `progress_assessment`: The output from Stage 4

## Custom Variables

You can provide custom variables when running a stage:

```bash
rust-ai-toolkit run-stage -s 1 -p proj_12345 -v "key1=value1,key2=value2"
```

These variables will be available in the template as `{{key1}}` and `{{key2}}`.

## Customizing Templates

### Modifying Existing Templates

To modify an existing template:

1. Export the template to edit:
   ```bash
   rust-ai-toolkit export-template -n stage1 -o ./my-templates/
   ```

2. Edit the template file (`./my-templates/stage1.hbs`)

3. Import the modified template:
   ```bash
   rust-ai-toolkit import-template -n stage1 -f ./my-templates/stage1.hbs
   ```

### Creating New Templates

To create a new template:

1. Create a Handlebars template file (e.g., `my-custom-template.hbs`)

2. Import it into the toolkit:
   ```bash
   rust-ai-toolkit import-template -n my-custom-template -f ./my-custom-template.hbs
   ```

3. Use it with a stage:
   ```bash
   rust-ai-toolkit run-stage -s 1 -p proj_12345 -t my-custom-template
   ```

## Template Examples

### Basic Project Planning Template

```handlebars
# Project Plan for {{project_name}}

## Project Description
{{project_description}}

## Requirements
Please analyze this project and provide:

1. Core features that should be implemented
2. Technical stack recommendations
3. Implementation timeline
4. Potential challenges and solutions

Please be detailed and specific in your analysis.
```

### Architecture Design with Conditionals

```handlebars
# Architecture Design for {{project_name}}

## Project Overview
{{project_description}}

## Design Requirements
Please create a detailed architecture design that includes:

1. System components and their interactions
2. Data models and database schema
3. API endpoints and their functionality
{{#if is_web_app}}
4. Frontend architecture and user flows
5. Responsive design considerations
{{else}}
4. Command-line interface design
5. Configuration options
{{/if}}
6. Deployment strategy

Please provide diagrams and explanations for each component.
```

### Implementation Strategy with Iterations

```handlebars
# Implementation Strategy for {{project_name}}

## Project Overview
{{project_description}}

## Development Phases

{{#each phases}}
### Phase {{@index}}: {{this.name}}
Duration: {{this.duration}}

#### Goals:
{{#each this.goals}}
- {{this}}
{{/each}}

#### Deliverables:
{{#each this.deliverables}}
- {{this}}
{{/each}}

{{/each}}

## Testing Strategy
Please outline a comprehensive testing strategy for this project.
```

## Advanced Template Techniques

### Helper Functions

Handlebars supports helper functions for more complex transformations:

```handlebars
{{#if (eq variable "value")}}
  This will be rendered if variable equals "value".
{{/if}}
```

### Partials

You can create reusable template fragments:

```handlebars
{{> common_header}}

# Main content here

{{> common_footer}}
```

To use partials, create separate template files for each partial and reference them with the `>` syntax.

### Escaping

By default, HTML in variables is escaped. To prevent this:

```handlebars
{{{variable_with_html}}}
```

Note the triple braces instead of double.

## Best Practices

1. **Start with the default templates** and make incremental changes
2. **Keep prompts focused** on specific tasks for better results
3. **Use clear instructions** at the beginning of templates
4. **Structure output format** to make parsing easier
5. **Include examples** in your prompts when possible
6. **Test templates** with different inputs to ensure they work as expected

## Troubleshooting

### Template Not Found

If you get a "Template not found" error:
1. Check that the template name is correct
2. Verify the template exists in `~/.rust-ai-toolkit/templates/`
3. Try exporting and reimporting the template

### Variable Not Rendered

If a variable is not being replaced:
1. Check the variable name for typos
2. Verify the variable is being passed correctly
3. Check for proper syntax (`{{variable}}` not `{{ variable }}`)

### Syntax Errors

If you encounter syntax errors:
1. Validate your Handlebars syntax
2. Check for unclosed tags or blocks
3. Verify that conditionals and loops are properly closed

## Resetting Templates

To reset a template to its default version:

```bash
rust-ai-toolkit reset-template -n stage1
```

To reset all templates:

```bash
rust-ai-toolkit reset-templates --all
```

## Template Versioning

It's a good practice to version your templates:

1. Export all templates before making significant changes
2. Use version comments at the top of templates
3. Consider using a version control system for your template directory

## Example: Complete Workflow

```bash
# Export the default template
rust-ai-toolkit export-template -n stage1 -o ./templates/

# Edit the template in your favorite editor
vim ./templates/stage1.hbs

# Import the modified template
rust-ai-toolkit import-template -n stage1 -f ./templates/stage1.hbs

# Run a stage with the modified template
rust-ai-toolkit run-stage -s 1 -p proj_12345

# If needed, reset to the default template
rust-ai-toolkit reset-template -n stage1
```

## Further Resources

For more information on Handlebars syntax, visit the [Handlebars documentation](https://handlebarsjs.com/guide/). 