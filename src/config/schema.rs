//! JSON configuration file schema.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::cli::ProbeConfig;

/// Top-level configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub probe: ProbeSettings,
    #[serde(default)]
    pub logging: LoggingSettings,
    #[serde(default)]
    pub ui: UiSettings,
    pub hosts: Vec<HostConfig>,
}

/// Probe timing and behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSettings {
    /// Probe interval in seconds (1-60 inclusive)
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    /// Probe timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Rolling window duration in minutes
    #[serde(default = "default_window")]
    pub window_minutes: u64,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    #[serde(default = "default_log_level")]
    pub level: String,
}

/// UI display settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_ms: u64,
    #[serde(default = "default_false")]
    pub show_error_details: bool,
}

/// Host configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub name: String,
    pub address: String,
    pub port: u16,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// Default value functions
fn default_version() -> String {
    "1.0".to_string()
}
const fn default_interval() -> u64 {
    5
}
const fn default_timeout() -> u64 {
    3
}
const fn default_window() -> u64 {
    10
}
fn default_log_level() -> String {
    "info".to_string()
}
const fn default_refresh_rate() -> u64 {
    100
}
const fn default_true() -> bool {
    true
}
const fn default_false() -> bool {
    false
}

impl Default for ProbeSettings {
    fn default() -> Self {
        Self {
            interval_seconds: default_interval(),
            timeout_seconds: default_timeout(),
            window_minutes: default_window(),
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            refresh_rate_ms: default_refresh_rate(),
            show_error_details: default_false(),
        }
    }
}

impl ConfigFile {
    /// Load configuration from file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Use new validator with helpful error messages
        super::validation::ConfigValidator::validate(&config)
            .with_context(|| format!("Configuration validation failed for: {}", path.display()))?;

        Ok(config)
    }

    /// Save configuration to file with pretty formatting.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(path, json)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Convert to internal `ProbeConfig`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn to_probe_config(&self) -> ProbeConfig {
        ProbeConfig::new(
            Duration::from_secs(self.probe.interval_seconds),
            Duration::from_secs(self.probe.timeout_seconds),
            Duration::from_secs(self.probe.window_minutes * 60),
            (self.probe.window_minutes * 60) as usize,
        )
    }

    /// Get enabled hosts as (name, address:port) tuples.
    #[must_use]
    pub fn get_hosts(&self) -> Vec<(String, String)> {
        self.hosts
            .iter()
            .filter(|h| h.enabled)
            .map(|h| (h.name.clone(), format!("{}:{}", h.address, h.port)))
            .collect()
    }

    /// Create a default configuration with example hosts.
    pub fn default_config() -> Self {
        Self {
            version: default_version(),
            probe: ProbeSettings::default(),
            logging: LoggingSettings::default(),
            ui: UiSettings::default(),
            hosts: vec![HostConfig {
                name: "Google HTTPS".to_string(),
                address: "www.google.com".to_string(),
                port: 443,
                enabled: true,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::validation::ConfigValidator;

    #[test]
    fn test_default_config() {
        let config = ConfigFile::default_config();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.probe.interval_seconds, 5);
        assert_eq!(config.probe.timeout_seconds, 3);
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].name, "Google HTTPS");
        assert_eq!(config.hosts[0].address, "www.google.com");
        assert_eq!(config.hosts[0].port, 443);
    }

    #[test]
    fn test_config_validation_no_hosts() {
        let mut config = ConfigFile::default_config();
        config.hosts.clear();
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_config_validation_too_many_hosts() {
        let mut config = ConfigFile::default_config();
        for i in 0..6 {
            config.hosts.push(HostConfig {
                name: format!("Host {}", i),
                address: "127.0.0.1".to_string(),
                port: 8080 + i as u16,
                enabled: true,
            });
        }
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_config_validation_timeout_ge_interval() {
        let mut config = ConfigFile::default_config();
        config.probe.timeout_seconds = 10;
        config.probe.interval_seconds = 5;
        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_get_hosts() {
        let config = ConfigFile::default_config();
        let hosts = config.get_hosts();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].0, "Google HTTPS");
        assert_eq!(hosts[0].1, "www.google.com:443");
    }

    #[test]
    fn test_get_hosts_filters_disabled() {
        let mut config = ConfigFile::default_config();
        // Add a second host and disable it
        config.hosts.push(HostConfig {
            name: "Cloudflare DNS".to_string(),
            address: "1.1.1.1".to_string(),
            port: 53,
            enabled: false,
        });
        let hosts = config.get_hosts();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].0, "Google HTTPS");
    }

    #[test]
    fn test_to_probe_config() {
        let config = ConfigFile::default_config();
        let probe_config = config.to_probe_config();
        assert_eq!(probe_config.interval, Duration::from_secs(5));
        assert_eq!(probe_config.timeout, Duration::from_secs(3));
        assert_eq!(probe_config.window_duration, Duration::from_secs(600));
    }

    #[test]
    fn test_json_round_trip() {
        let config = ConfigFile::default_config();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: ConfigFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.hosts.len(), config.hosts.len());
        assert_eq!(parsed.probe.interval_seconds, config.probe.interval_seconds);
    }

    #[test]
    fn test_config_validation_interval_too_low() {
        let mut config = ConfigFile::default_config();
        config.probe.interval_seconds = 0;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 1 and 60"));
    }

    #[test]
    fn test_config_validation_interval_too_high() {
        let mut config = ConfigFile::default_config();
        config.probe.interval_seconds = 61;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 1 and 60"));
    }

    #[test]
    fn test_config_validation_interval_at_boundaries() {
        let mut config = ConfigFile::default_config();

        // Test lower boundary (1 second)
        config.probe.interval_seconds = 1;
        config.probe.timeout_seconds = 0; // Timeout < interval
        assert!(ConfigValidator::validate(&config).is_ok());

        // Test upper boundary (60 seconds)
        config.probe.interval_seconds = 60;
        config.probe.timeout_seconds = 59; // Timeout < interval
        assert!(ConfigValidator::validate(&config).is_ok());
    }
}
