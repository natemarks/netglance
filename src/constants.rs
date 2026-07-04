//! Global constants for the application.
//!
//! All configurable constants are defined here in one place for easy maintenance.

use std::time::Duration;

// =============================================================================
// METRICS LOGGING CONFIGURATION
// =============================================================================

/// How frequently to write metrics snapshots to the JSONL log file.
pub const METRICS_LOG_INTERVAL_SECS: u64 = 10;

// =============================================================================
// LOG ROTATION CONFIGURATION
// =============================================================================

/// Age threshold for log rotation (older logs are compressed).
///
/// Logs older than this duration will be compressed to .tgz format
/// when the application starts.
pub const LOG_ROTATION_THRESHOLD_HOURS: u64 = 24;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get the metrics logging interval as a Duration.
pub fn metrics_log_interval() -> Duration {
    Duration::from_secs(METRICS_LOG_INTERVAL_SECS)
}

/// Get the log rotation threshold as a Duration.
pub fn log_rotation_threshold() -> Duration {
    Duration::from_secs(LOG_ROTATION_THRESHOLD_HOURS * 3600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_reasonable() {
        // Validate at compile time that constants are reasonable
        const { assert!(METRICS_LOG_INTERVAL_SECS > 0) };
        const { assert!(LOG_ROTATION_THRESHOLD_HOURS > 0) };
    }

    #[test]
    fn test_helper_functions() {
        assert_eq!(metrics_log_interval(), Duration::from_secs(10));
        assert_eq!(log_rotation_threshold(), Duration::from_secs(24 * 3600));
    }
}
