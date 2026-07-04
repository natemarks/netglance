//! Configuration management.
//!
//! This module handles both CLI arguments and JSON configuration files.

pub mod cli;
pub mod directory;
pub mod schema;
pub mod validation;

// Re-export commonly used types
pub use cli::ProbeConfig;
pub use directory::{default_config_path, ensure_config_dir, logs_dir};
pub use schema::ConfigFile;
