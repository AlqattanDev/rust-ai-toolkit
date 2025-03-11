# Troubleshooting Guide

## Overview

This guide addresses common issues you might encounter when using the Rust AI Toolkit and provides solutions to resolve them. If you're experiencing a problem not covered here, please check our GitHub issues or create a new one.

## Installation Issues

### Cargo Install Fails

**Problem**: `cargo install rust-ai-toolkit` fails with compilation errors.

**Solutions**:

1. **Update Rust**:
   ```bash
   rustup update
   ```

2. **Check dependencies**:
   Ensure you have the required system dependencies:
   - OpenSSL development libraries
   - pkg-config
   
   On Ubuntu/Debian:
   ```bash
   sudo apt install pkg-config libssl-dev
   ```
   
   On macOS:
   ```bash
   brew install openssl pkg-config
   ```
   
   On Windows:
   Install OpenSSL using a package manager like Chocolatey or manually.

3. **Try with specific version**:
   ```bash
   cargo install rust-ai-toolkit --version 0.1.0
   ```

### Missing Executable

**Problem**: After installation, the `rust-ai-toolkit` command is not found.

**Solutions**:

1. **Check PATH**:
   Ensure Cargo's bin directory is in your PATH:
   ```bash
   echo $PATH | grep -q ~/.cargo/bin || echo "~/.cargo/bin is not in PATH"
   ```

2. **Manual path addition**:
   Add to your shell profile (`.bashrc`, `.zshrc`, etc.):
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

3. **Reinstall**:
   ```bash
   cargo install --force rust-ai-toolkit
   ```

## Configuration Issues

### API Key Problems

**Problem**: Authentication errors with AI providers.

**Solutions**:

1. **Verify API key**:
   Check that your API key is correct and active:
   ```bash
   rust-ai-toolkit config --show
   ```

2. **Update API key**:
   ```bash
   rust-ai-toolkit config --provider openai --api-key "your-new-key"
   ```

3. **Check provider status**:
   Visit the provider's status page to ensure their services are operational:
   - [OpenAI Status](https://status.openai.com/)
   - [Anthropic Status](https://status.anthropic.com/)

4. **Permissions**:
   Ensure your API key has the necessary permissions and quota for the models you're trying to use.

### Configuration File Corruption

**Problem**: Configuration file is corrupted or missing.

**Solutions**:

1. **Reset configuration**:
   ```bash
   rust-ai-toolkit config --reset
   ```

2. **Manual edit**:
   Edit the configuration file directly:
   ```bash
   vim ~/.rust-ai-toolkit/config.toml
   ```

3. **Check file permissions**:
   ```bash
   ls -la ~/.rust-ai-toolkit/
   ```

## Project Management Issues

### Project Creation Fails

**Problem**: Unable to create a new project.

**Solutions**:

1. **Check directory permissions**:
   Ensure you have write permissions to the current directory.

2. **Specify a different path**:
   ```bash
   rust-ai-toolkit init -n "Project Name" -d "Description" -p /path/to/writable/directory
   ```

3. **Check disk space**:
   Ensure you have sufficient disk space.

### Project Not Found

**Problem**: "Project not found" error when trying to run a stage.

**Solutions**:

1. **List all projects**:
   ```bash
   rust-ai-toolkit list
   ```

2. **Check project ID**:
   Verify you're using the correct project ID.

3. **Project database repair**:
   ```bash
   rust-ai-toolkit repair-db
   ```

### Stage Execution Fails

**Problem**: A stage fails to execute properly.

**Solutions**:

1. **Check prerequisites**:
   Ensure all prerequisite stages have been completed:
   ```bash
   rust-ai-toolkit status -p <PROJECT_ID>
   ```

2. **Run with verbose logging**:
   ```bash
   rust-ai-toolkit run-stage -s <STAGE> -p <PROJECT_ID> --verbose
   ```

3. **Reset the stage**:
   ```bash
   rust-ai-toolkit reset-stage -s <STAGE> -p <PROJECT_ID>
   ```

## AI Provider Issues

### Rate Limiting

**Problem**: Hitting rate limits with your AI provider.

**Solutions**:

1. **Adjust rate limits**:
   ```bash
   rust-ai-toolkit config --rate-limit 20  # Reduce to 20 requests per minute
   ```

2. **Enable exponential backoff**:
   ```bash
   rust-ai-toolkit config --backoff-factor 2.0
   ```

3. **Use caching**:
   ```bash
   rust-ai-toolkit config --cache-enabled true
   ```

4. **Switch providers**:
   ```bash
   rust-ai-toolkit config --provider anthropic
   ```

### Timeout Errors

**Problem**: Requests to AI providers time out.

**Solutions**:

1. **Increase timeout**:
   ```bash
   rust-ai-toolkit config --request-timeout 120  # 120 seconds
   ```

2. **Check network**:
   Ensure you have a stable internet connection.

3. **Use a smaller model**:
   ```bash
   rust-ai-toolkit config --model "gpt-3.5-turbo"  # Instead of GPT-4
   ```

### Poor Quality Responses

**Problem**: AI responses are low quality or irrelevant.

**Solutions**:

1. **Use a more capable model**:
   ```bash
   rust-ai-toolkit config --model "gpt-4" # Or "claude-3-opus-20240229"
   ```

2. **Customize templates**:
   Improve the prompts to give better instructions:
   ```bash
   rust-ai-toolkit export-template -n stage1 -o ./
   # Edit stage1.hbs
   rust-ai-toolkit import-template -n stage1 -f ./stage1.hbs
   ```

3. **Provide more context**:
   Add more detailed project descriptions or custom variables.

## Template Issues

### Template Not Found

**Problem**: "Template not found" error when running a stage.

**Solutions**:

1. **List available templates**:
   ```bash
   rust-ai-toolkit list-templates
   ```

2. **Reset to default templates**:
   ```bash
   rust-ai-toolkit reset-templates --all
   ```

3. **Check template directory**:
   ```bash
   ls -la ~/.rust-ai-toolkit/templates/
   ```

### Template Syntax Errors

**Problem**: Errors related to template syntax.

**Solutions**:

1. **Validate template syntax**:
   ```bash
   rust-ai-toolkit validate-template -n <TEMPLATE_NAME>
   ```

2. **Reset specific template**:
   ```bash
   rust-ai-toolkit reset-template -n <TEMPLATE_NAME>
   ```

3. **Check Handlebars syntax**:
   Ensure all opening tags have corresponding closing tags.

### Variables Not Rendering

**Problem**: Template variables are not being replaced with values.

**Solutions**:

1. **Check variable names**:
   Ensure variable names in the template match the ones being provided.

2. **Verify variable format**:
   Use `{{variable}}` not `{{ variable }}` or `{variable}`.

3. **Debug template rendering**:
   ```bash
   rust-ai-toolkit debug-template -n <TEMPLATE_NAME> -p <PROJECT_ID>
   ```

## Performance Issues

### Slow Response Times

**Problem**: The toolkit is slow to respond or generate content.

**Solutions**:

1. **Enable caching**:
   ```bash
   rust-ai-toolkit config --cache-enabled true
   ```

2. **Use a faster model**:
   ```bash
   rust-ai-toolkit config --model "claude-3-haiku-20240307"
   ```

3. **Optimize templates**:
   Shorter, more focused prompts generally get faster responses.

### High API Costs

**Problem**: Using the toolkit results in high API costs.

**Solutions**:

1. **Enable aggressive caching**:
   ```bash
   rust-ai-toolkit config --cache-enabled true --cache-ttl 168  # 1 week
   ```

2. **Use cheaper models**:
   ```bash
   rust-ai-toolkit config --model "gpt-3.5-turbo"  # Instead of GPT-4
   ```

3. **Limit token usage**:
   ```bash
   rust-ai-toolkit config --max-tokens 1000
   ```

4. **Monitor usage**:
   ```bash
   rust-ai-toolkit usage-stats
   ```

## File System Issues

### Permission Denied

**Problem**: Permission denied errors when accessing files.

**Solutions**:

1. **Check file permissions**:
   ```bash
   ls -la ~/.rust-ai-toolkit/
   ```

2. **Fix permissions**:
   ```bash
   chmod -R u+rw ~/.rust-ai-toolkit/
   ```

3. **Run with elevated privileges** (not recommended for regular use):
   ```bash
   sudo rust-ai-toolkit <command>
   ```

### Disk Space Issues

**Problem**: Running out of disk space.

**Solutions**:

1. **Clear caches**:
   ```bash
   rust-ai-toolkit clear-cache --all
   ```

2. **Check space usage**:
   ```bash
   du -sh ~/.rust-ai-toolkit/
   ```

3. **Configure smaller cache size**:
   ```bash
   rust-ai-toolkit config --max-cache-size 100  # 100 MB
   ```

## Advanced Troubleshooting

### Diagnostic Mode

Run the toolkit in diagnostic mode to get detailed information:

```bash
rust-ai-toolkit diagnose
```

This will check:
- Configuration validity
- API connectivity
- File system permissions
- Template integrity

### Logs

Check the logs for detailed error information:

```bash
cat ~/.rust-ai-toolkit/logs/toolkit.log
```

Enable debug logging:

```bash
rust-ai-toolkit config --log-level debug
```

### Reset Everything

As a last resort, you can reset everything:

```bash
rust-ai-toolkit reset --all
```

This will:
- Reset all configuration to defaults
- Clear all caches
- Reset all templates
- Preserve your projects

## Getting Help

If you're still experiencing issues:

1. **Check GitHub Issues**:
   Browse existing issues or create a new one on our GitHub repository.

2. **Community Forum**:
   Join our community forum to ask questions and share experiences.

3. **Documentation**:
   Review the full documentation for more detailed information:
   - [Usage Guide](USAGE.md)
   - [API Provider Configuration](API_PROVIDERS.md)
   - [Template Customization](TEMPLATES.md) 