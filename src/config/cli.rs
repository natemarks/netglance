//! Configuration for TCP probing behavior.

use std::time::Duration;

/// Configuration for TCP probe behavior.
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    /// How often to probe each host
    pub interval: Duration,
    /// Maximum time to wait for a connection
    pub timeout: Duration,
    /// How long to keep probe results
    pub window_duration: Duration,
    /// Maximum samples to store per host
    pub max_samples_per_host: usize,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            timeout: Duration::from_secs(5),
            window_duration: Duration::from_mins(10),
            max_samples_per_host: 600,
        }
    }
}

impl ProbeConfig {
    /// Create a new configuration with custom values.
    #[must_use]
    pub const fn new(
        interval: Duration,
        timeout: Duration,
        window_duration: Duration,
        max_samples_per_host: usize,
    ) -> Self {
        Self {
            interval,
            timeout,
            window_duration,
            max_samples_per_host,
        }
    }

    /// Validate the configuration and return warnings.
    ///
    /// Returns a list of warning messages if configuration values might cause issues.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.timeout >= self.interval {
            warnings.push(format!(
                "Timeout ({:?}) should be less than interval ({:?}) to avoid overlapping probes",
                self.timeout, self.interval
            ));
        }

        if self.max_samples_per_host < 10 {
            warnings.push(format!(
                "Very low max_samples ({}) may not provide enough data for statistics",
                self.max_samples_per_host
            ));
        }

        let expected_samples = self.window_duration.as_secs() / self.interval.as_secs();
        if expected_samples > self.max_samples_per_host as u64 {
            warnings.push(format!(
                "Window duration ({:?}) with interval ({:?}) would generate {} samples, but max_samples is {}",
                self.window_duration, self.interval, expected_samples, self.max_samples_per_host
            ));
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ProbeConfig::default();
        assert_eq!(config.interval, Duration::from_secs(1));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.window_duration, Duration::from_secs(600));
        assert_eq!(config.max_samples_per_host, 600);
    }

    #[test]
    fn custom_config() {
        let config = ProbeConfig::new(
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_secs(300),
            150,
        );
        assert_eq!(config.interval, Duration::from_secs(2));
        assert_eq!(config.timeout, Duration::from_secs(3));
    }

    #[test]
    fn validate_good_config() {
        // Create a config that should pass validation
        let config = ProbeConfig::new(
            Duration::from_secs(2),
            Duration::from_secs(1), // timeout < interval
            Duration::from_secs(600),
            600,
        );
        let warnings = config.validate();
        assert!(
            warnings.is_empty(),
            "Expected no warnings, got: {:?}",
            warnings
        );
    }

    #[test]
    fn validate_timeout_warning() {
        let config = ProbeConfig::new(
            Duration::from_secs(1),
            Duration::from_secs(2), // timeout > interval
            Duration::from_secs(600),
            600,
        );
        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Timeout"));
    }

    #[test]
    fn validate_low_samples_warning() {
        let config = ProbeConfig::new(
            Duration::from_secs(1),
            Duration::from_millis(500),
            Duration::from_secs(600),
            5, // very low
        );
        let warnings = config.validate();
        assert!(warnings.iter().any(|w| w.contains("Very low max_samples")));
    }

    #[test]
    fn validate_insufficient_samples_warning() {
        let config = ProbeConfig::new(
            Duration::from_secs(1),
            Duration::from_millis(500),
            Duration::from_secs(1000), // 1000 seconds
            100,                       // but only 100 samples max
        );
        let warnings = config.validate();
        assert!(warnings
            .iter()
            .any(|w| w.contains("would generate") && w.contains("samples")));
    }
}
