use rust_ai_toolkit::error::ToolkitError;
use rust_ai_toolkit::config::Config;  // Assuming this exists
use crate::common::{TestFixture, run_async};

#[test]
fn test_config_creation() {
    let fixture = TestFixture::new();
    
    // Test config creation
    let result = run_async(async {
        Config::new(&fixture.config_path).await
    });
    
    assert!(result.is_ok());
}

#[test]
fn test_invalid_config() {
    let fixture = TestFixture::new();
    
    // Write invalid config
    std::fs::write(
        &fixture.config_path,
        "invalid_toml_content"
    ).expect("Failed to write test file");
    
    // Test config loading
    let result = run_async(async {
        Config::new(&fixture.config_path).await
    });
    
    assert!(matches!(result, Err(ToolkitError::Config(_))));
} 