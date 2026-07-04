//! Configuration directory management.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::{env, fs};

/// Get the configuration directory path.
///
/// Returns `$HOME/.netglance/`
pub fn config_dir() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".netglance"))
}

/// Ensure configuration directory structure exists.
///
/// Creates:
/// - `$HOME/.netglance/`
/// - `$HOME/.netglance/logs/`
///
/// Returns the config directory path.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir()?;

    // Create main config directory
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory: {}", dir.display()))?;
    }

    // Create logs subdirectory
    let log_dir = dir.join("logs");
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;
    }

    Ok(dir)
}

/// Get the default configuration file path.
///
/// Returns `$HOME/.netglance/settings.json`
pub fn default_config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("settings.json"))
}

/// Get the logs directory path.
///
/// Returns `$HOME/.netglance/logs/`
pub fn logs_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("logs"))
}

/// Check if default configuration file exists.
#[must_use]
#[allow(dead_code)]
pub fn config_exists() -> bool {
    default_config_path().is_ok_and(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_dir() {
        let home = env::var("HOME").unwrap();
        let dir = config_dir().unwrap();
        assert_eq!(dir, PathBuf::from(home).join(".netglance"));
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path().unwrap();
        assert!(path.to_string_lossy().ends_with(".netglance/settings.json"));
    }

    #[test]
    fn test_logs_dir() {
        let path = logs_dir().unwrap();
        assert!(path.to_string_lossy().ends_with(".netglance/logs"));
    }
}
