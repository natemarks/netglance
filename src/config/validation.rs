//! Configuration validation with helpful error messages.

use anyhow::{bail, Result};
use super::schema::ConfigFile;

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate(config: &ConfigFile) -> Result<()> {
        Self::validate_probe_settings(config)?;
        Self::validate_hosts(config)?;
        Self::validate_logging(config)?;
        Ok(())
    }

    fn validate_probe_settings(config: &ConfigFile) -> Result<()> {
        let probe = &config.probe;

        // Interval range check
        if !(1..=60).contains(&probe.interval_seconds) {
            bail!(
                "Invalid probe interval: {}s\n\
                 Must be between 1 and 60 seconds.\n\
                 \n\
                 Edit your config file and set probe.interval_seconds to a value like 5.",
                probe.interval_seconds
            );
        }

        // Timeout vs interval check
        if probe.timeout_seconds >= probe.interval_seconds {
            bail!(
                "Invalid timing configuration:\n\
                 - probe.timeout_seconds: {}s\n\
                 - probe.interval_seconds: {}s\n\
                 \n\
                 Timeout must be less than interval to avoid overlapping probes.\n\
                 Suggestion: Set timeout_seconds to {} and interval_seconds to {}.",
                probe.timeout_seconds,
                probe.interval_seconds,
                probe.interval_seconds - 1,
                probe.interval_seconds
            );
        }

        Ok(())
    }

    fn validate_hosts(config: &ConfigFile) -> Result<()> {
        if config.hosts.is_empty() {
            bail!(
                "No hosts configured!\n\
                 \n\
                 Add hosts to your config file:\n\
                 {{\n\
                   \"hosts\": [\n\
                     {{\n\
                       \"name\": \"Google DNS\",\n\
                       \"address\": \"8.8.8.8\",\n\
                       \"port\": 53,\n\
                       \"enabled\": true\n\
                     }}\n\
                   ]\n\
                 }}"
            );
        }

        if config.hosts.len() > 5 {
            bail!(
                "Too many hosts configured: {}\n\
                 Maximum allowed: 5\n\
                 \n\
                 Remove {} host(s) from your config file.",
                config.hosts.len(),
                config.hosts.len() - 5
            );
        }

        // Validate each host
        for (idx, host) in config.hosts.iter().enumerate() {
            if host.name.is_empty() {
                bail!("Host #{} has empty name. Please provide a descriptive name.", idx + 1);
            }

            if host.address.is_empty() {
                bail!("Host #{} ({}) has empty address.", idx + 1, host.name);
            }

            if host.port == 0 {
                bail!(
                    "Host #{} ({}) has invalid port: {}\n\
                     Port must be between 1 and 65535.",
                    idx + 1, host.name, host.port
                );
            }
        }

        Ok(())
    }

    fn validate_logging(config: &ConfigFile) -> Result<()> {
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&config.logging.level.as_str()) {
            bail!(
                "Invalid log level: \"{}\"\n\
                 Valid options: error, warn, info, debug, trace\n\
                 \n\
                 Recommendation: Use \"info\" for normal operation.",
                config.logging.level
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{HostConfig, LoggingSettings, ProbeSettings};

    use crate::config::schema::UiSettings;

    fn make_valid_config() -> ConfigFile {
        ConfigFile {
            version: "1.0".to_string(),
            probe: ProbeSettings {
                interval_seconds: 5,
                timeout_seconds: 3,
                window_minutes: 10,
            },
            hosts: vec![
                HostConfig {
                    name: "Test Host".to_string(),
                    address: "127.0.0.1".to_string(),
                    port: 80,
                    enabled: true,
                },
            ],
            logging: LoggingSettings {
                level: "info".to_string(),
            },
            ui: UiSettings::default(),
        }
    }

    #[test]
    fn test_valid_config() {
        let config = make_valid_config();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_interval_too_low() {
        let mut config = make_valid_config();
        config.probe.interval_seconds = 0;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid probe interval"));
    }

    #[test]
    fn test_interval_too_high() {
        let mut config = make_valid_config();
        config.probe.interval_seconds = 61;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid probe interval"));
    }

    #[test]
    fn test_timeout_exceeds_interval() {
        let mut config = make_valid_config();
        config.probe.timeout_seconds = 10;
        config.probe.interval_seconds = 5;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid timing configuration"));
    }

    #[test]
    fn test_no_hosts() {
        let mut config = make_valid_config();
        config.hosts.clear();
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No hosts configured"));
    }

    #[test]
    fn test_too_many_hosts() {
        let mut config = make_valid_config();
        for i in 0..6 {
            config.hosts.push(HostConfig {
                name: format!("Host {}", i),
                address: "127.0.0.1".to_string(),
                port: 80 + i,
                enabled: true,
            });
        }
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many hosts"));
    }

    #[test]
    fn test_invalid_port() {
        let mut config = make_valid_config();
        config.hosts[0].port = 0;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid port"));
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = make_valid_config();
        config.logging.level = "invalid".to_string();
        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid log level"));
    }
}
