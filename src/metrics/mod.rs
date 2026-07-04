//! Metrics collection and analysis for TCP connection monitoring.
//!
//! This module provides the data structures and logic for tracking TCP probe results,
//! calculating statistics, and determining host health status.

use std::fmt;
use std::time::{Duration, Instant};

pub mod host_state;
pub mod overall;
pub mod ring_buffer;
pub mod statistics;
pub mod status;
pub mod store;
pub mod updater;

// Public re-exports for library API
pub use host_state::HostState;
pub use overall::OverallMetrics;
pub use statistics::HostMetrics;
pub use status::HostStatus;
#[allow(unused_imports)]
pub use store::{HostSnapshot, MetricsSnapshot, MetricsStore};
pub use updater::MetricsUpdater;

// Internal only
pub(crate) use ring_buffer::RollingWindow;

/// Result of a single TCP connection probe.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// When the probe was executed
    pub timestamp: Instant,
    /// Latency if successful, None if failed
    pub latency: Option<Duration>,
    /// Error details if the probe failed
    pub error: Option<ProbeError>,
}

impl ProbeResult {
    /// Create a successful probe result
    pub fn success(timestamp: Instant, latency: Duration) -> Self {
        Self {
            timestamp,
            latency: Some(latency),
            error: None,
        }
    }

    /// Create a failed probe result
    pub fn failure(timestamp: Instant, error: ProbeError) -> Self {
        Self {
            timestamp,
            latency: None,
            error: Some(error),
        }
    }

    /// Check if the probe was successful
    pub fn is_success(&self) -> bool {
        self.latency.is_some()
    }

    /// Check if the probe failed
    pub fn is_failure(&self) -> bool {
        self.error.is_some()
    }
}

/// Errors that can occur during TCP probing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    /// Connection attempt timed out
    Timeout,
    /// Server actively refused the connection
    ConnectionRefused,
    /// Failed to resolve hostname
    DnsFailure,
    /// Other I/O error
    IoError(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::Timeout => write!(f, "Connection timeout"),
            ProbeError::ConnectionRefused => write!(f, "Connection refused"),
            ProbeError::DnsFailure => write!(f, "DNS resolution failed"),
            ProbeError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ProbeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_result_success() {
        let now = Instant::now();
        let latency = Duration::from_millis(42);
        let result = ProbeResult::success(now, latency);

        assert!(result.is_success());
        assert!(!result.is_failure());
        assert_eq!(result.latency, Some(latency));
        assert_eq!(result.error, None);
    }

    #[test]
    fn probe_result_failure() {
        let now = Instant::now();
        let error = ProbeError::Timeout;
        let result = ProbeResult::failure(now, error.clone());

        assert!(!result.is_success());
        assert!(result.is_failure());
        assert_eq!(result.latency, None);
        assert_eq!(result.error, Some(error));
    }

    #[test]
    fn probe_error_display() {
        assert_eq!(ProbeError::Timeout.to_string(), "Connection timeout");
        assert_eq!(
            ProbeError::ConnectionRefused.to_string(),
            "Connection refused"
        );
        assert_eq!(ProbeError::DnsFailure.to_string(), "DNS resolution failed");
        assert_eq!(
            ProbeError::IoError("test error".to_string()).to_string(),
            "I/O error: test error"
        );
    }
}
