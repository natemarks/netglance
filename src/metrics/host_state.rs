//! Host state management combining probe history and metrics.

use std::time::{Duration, Instant};

use super::{HostMetrics, HostStatus, ProbeResult, RollingWindow};

/// State for a single monitored host.
///
/// Combines the probe history (rolling window) with calculated metrics
/// and provides methods for updating and querying host status.
#[derive(Debug)]
pub struct HostState {
    /// Display name for the host
    hostname: String,
    /// Original address as configured (may be hostname:port or IP:port)
    address: String,
    /// Resolved IP address for probing (IP:port)
    resolved_address: String,
    /// Rolling window of probe results
    window: RollingWindow,
    /// Cached metrics calculated from the window
    current_metrics: HostMetrics,
}

impl HostState {
    /// Create a new host state.
    ///
    /// # Arguments
    /// * `hostname` - Display name (e.g., "api-1", "db-primary")
    /// * `address` - Connection address (e.g., "192.168.1.10:443", "api.example.com:80")
    /// * `resolved_address` - Resolved IP address for probing
    #[allow(dead_code)]
    pub fn new(hostname: String, address: String, resolved_address: String) -> Self {
        Self::with_window_config(
            hostname,
            address,
            resolved_address,
            Duration::from_secs(600), // 10 minutes
            600,                      // Max 600 samples
        )
    }

    /// Create a new host state with custom window configuration.
    pub fn with_window_config(
        hostname: String,
        address: String,
        resolved_address: String,
        window_duration: Duration,
        max_samples: usize,
    ) -> Self {
        Self {
            hostname,
            address,
            resolved_address,
            window: RollingWindow::new(window_duration, max_samples),
            current_metrics: HostMetrics::empty(),
        }
    }

    /// Add a probe result and update metrics.
    pub fn add_probe_result(&mut self, result: ProbeResult) {
        self.window.push(result);
        self.update_metrics();
    }

    /// Recalculate metrics from the current window.
    pub fn update_metrics(&mut self) {
        self.current_metrics = HostMetrics::from_window(&self.window);
    }

    /// Get the current metrics.
    pub fn metrics(&self) -> &HostMetrics {
        &self.current_metrics
    }

    /// Get the hostname.
    #[must_use]
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Get the original configured address.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get the resolved IP address for probing.
    #[must_use]
    pub fn resolved_address(&self) -> &str {
        &self.resolved_address
    }

    /// Determine the current health status.
    #[must_use]
    pub fn status(&self, now: Instant) -> HostStatus {
        let has_recent_data = self.current_metrics.has_recent_data(now);
        HostStatus::classify(
            self.current_metrics.success_rate,
            self.current_metrics.consecutive_failures,
            has_recent_data,
            self.current_metrics.total_probes,
        )
    }

    /// Get the number of samples in the window.
    #[must_use]
    pub fn sample_count(&self) -> usize {
        self.window.len()
    }

    /// Check if the host has any data.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_data(&self) -> bool {
        !self.window.is_empty()
    }

    /// Get recent samples for visualization (last N duration).
    #[must_use]
    pub fn recent_samples(&self, window: Duration, now: Instant) -> Vec<ProbeResult> {
        let cutoff = now.checked_sub(window).unwrap_or(now);
        self.window
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ProbeError;

    #[test]
    fn new_host_state() {
        let state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );

        assert_eq!(state.hostname(), "test-host");
        assert_eq!(state.address(), "localhost:8080");
        assert_eq!(state.resolved_address(), "127.0.0.1:8080");
        assert!(!state.has_data());
        assert_eq!(state.sample_count(), 0);
    }

    #[test]
    fn add_probe_result_updates_metrics() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let now = Instant::now();

        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(42)));

        assert!(state.has_data());
        assert_eq!(state.sample_count(), 1);
        assert_eq!(state.metrics().success_rate, 100.0);
        assert_eq!(state.metrics().avg_latency, Some(Duration::from_millis(42)));
    }

    #[test]
    fn status_healthy() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let now = Instant::now();

        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));
        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));
        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(30)));

        assert_eq!(state.status(now), HostStatus::Healthy);
    }

    #[test]
    fn status_degraded() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let now = Instant::now();

        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));
        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));
        state.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));

        assert_eq!(state.status(now), HostStatus::Degraded);
    }

    #[test]
    fn status_down() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let now = Instant::now();

        state.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));
        state.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));
        state.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));

        assert_eq!(state.status(now), HostStatus::Down);
    }

    #[test]
    fn status_stale() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let past = Instant::now() - Duration::from_secs(120);

        state.add_probe_result(ProbeResult::success(past, Duration::from_millis(10)));

        // Check status with current time
        let now = Instant::now();
        assert_eq!(state.status(now), HostStatus::Stale);
    }

    #[test]
    fn multiple_probes_tracked() {
        let mut state = HostState::new(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        );
        let now = Instant::now();

        for i in 0..10 {
            state.add_probe_result(ProbeResult::success(now, Duration::from_millis(10 + i * 5)));
        }

        assert_eq!(state.sample_count(), 10);
        assert_eq!(state.metrics().success_rate, 100.0);
        assert_eq!(state.metrics().min_latency, Some(Duration::from_millis(10)));
        assert_eq!(state.metrics().max_latency, Some(Duration::from_millis(55)));
    }

    #[test]
    fn window_expiration() {
        let mut state = HostState::with_window_config(
            "test-host".to_string(),
            "localhost:8080".to_string(),
            "127.0.0.1:8080".to_string(),
            Duration::from_secs(10), // 10 second window
            100,
        );

        let now = Instant::now();

        // Add old sample
        state.add_probe_result(ProbeResult::success(
            now - Duration::from_secs(20),
            Duration::from_millis(10),
        ));
        assert_eq!(state.sample_count(), 1);

        // Add new sample, which should expire the old one
        state.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));

        // Only new sample should remain
        assert_eq!(state.sample_count(), 1);
    }
}
