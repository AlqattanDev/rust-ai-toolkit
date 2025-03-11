use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Creates a temporary directory for test file operations
pub fn setup_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temporary directory")
}

/// Helper to get a path within the temporary directory
pub fn temp_path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(name)
}

/// Run async code in a synchronous test
pub fn run_async<F>(future: F) -> F::Output 
where
    F: std::future::Future,
{
    Runtime::new()
        .expect("Failed to create runtime")
        .block_on(future)
}

/// Test fixture for setting up a mock configuration
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub config_path: PathBuf,
}

impl TestFixture {
    pub fn new() -> Self {
        let temp_dir = setup_temp_dir();
        let config_path = temp_path(&temp_dir, "config.toml");
        Self {
            temp_dir,
            config_path,
        }
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for creating test data structures
#[macro_export]
macro_rules! test_struct {
    ($name:ident { $($field:ident: $value:expr),* $(,)? }) => {
        $name {
            $($field: $value,)*
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_creation() {
        let dir = setup_temp_dir();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_fixture_creation() {
        let fixture = TestFixture::new();
        assert!(fixture.temp_dir.path().exists());
    }
} 