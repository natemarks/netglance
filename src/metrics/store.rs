//! Central metrics storage for all monitored hosts.

use std::time::{Duration, Instant};

use super::{HostState, OverallMetrics, ProbeResult};
use crate::config::ProbeConfig;

/// Central storage for all host monitoring state.
///
/// Maintains a `HostState` for each monitored host and provides
/// methods for updating and querying metrics.
#[derive(Debug)]
pub struct MetricsStore {
    /// States for each monitored host
    hosts: Vec<HostState>,
    /// Configuration
    config: ProbeConfig,
}

impl MetricsStore {
    /// Create a new metrics store.
    ///
    /// # Arguments
    /// * `host_configs` - List of (hostname, address, resolved_address) tuples
    /// * `config` - Probe configuration
    pub fn new(host_configs: Vec<(String, String, String)>, config: ProbeConfig) -> Self {
        let hosts = host_configs
            .into_iter()
            .map(|(hostname, address, resolved_address)| {
                HostState::with_window_config(
                    hostname,
                    address,
                    resolved_address,
                    config.window_duration,
                    config.max_samples_per_host,
                )
            })
            .collect();

        Self { hosts, config }
    }

    /// Handle a probe result for a specific host.
    ///
    /// # Arguments
    /// * `host_idx` - Index of the host (from probe scheduler)
    /// * `result` - The probe result to add
    pub fn handle_probe_result(&mut self, host_idx: usize, result: ProbeResult) {
        if let Some(host) = self.hosts.get_mut(host_idx) {
            host.add_probe_result(result);
        }
    }

    /// Update metrics for all hosts.
    ///
    /// This recalculates statistics from the rolling windows.
    /// Should be called periodically or after adding probe results.
    pub fn update_all_metrics(&mut self) {
        for host in &mut self.hosts {
            host.update_metrics();
        }
    }

    /// Get a reference to a specific host's state.
    #[allow(dead_code)]
    pub fn get_host(&self, idx: usize) -> Option<&HostState> {
        self.hosts.get(idx)
    }

    /// Get a mutable reference to a specific host's state.
    #[allow(dead_code)]
    pub fn get_host_mut(&mut self, idx: usize) -> Option<&mut HostState> {
        self.hosts.get_mut(idx)
    }

    /// Get all hosts.
    #[allow(dead_code)]
    pub fn hosts(&self) -> &[HostState] {
        &self.hosts
    }

    /// Get the number of monitored hosts.
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Calculate overall metrics across all hosts.
    pub fn get_overall_metrics(&self, now: Instant) -> OverallMetrics {
        OverallMetrics::calculate(&self.hosts, now)
    }

    /// Get the configuration.
    pub fn config(&self) -> &ProbeConfig {
        &self.config
    }

    /// Create a snapshot of current metrics for rendering.
    pub fn snapshot(&self, now: Instant) -> MetricsSnapshot {
        MetricsSnapshot {
            hosts: self
                .hosts
                .iter()
                .map(|h| HostSnapshot::from_host(h, now))
                .collect(),
            overall: self.get_overall_metrics(now),
            timestamp: now,
        }
    }
}

/// Snapshot of metrics at a point in time.
///
/// This is an immutable copy of metrics data suitable for
/// passing to the UI thread without holding locks.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MetricsSnapshot {
    /// Snapshots of all hosts
    pub hosts: Vec<HostSnapshot>,
    /// Overall aggregated metrics
    pub overall: OverallMetrics,
    /// When this snapshot was taken
    pub timestamp: Instant,
}

/// Snapshot of a single host's state.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HostSnapshot {
    /// Display name
    pub hostname: String,
    /// Original configured address (may be hostname:port)
    pub address: String,
    /// Resolved IP address (IP:port)
    pub resolved_address: String,
    /// Current metrics
    pub metrics: super::HostMetrics,
    /// Health status
    pub status: super::HostStatus,
    /// Number of samples in window
    pub sample_count: usize,
    /// Recent samples for visualization (last 15 minutes)
    pub recent_samples: Vec<super::ProbeResult>,
}

impl HostSnapshot {
    /// Create a snapshot from a HostState.
    fn from_host(host: &HostState, now: Instant) -> Self {
        // Get last 15 minutes of samples for visualization
        let recent_samples = host.recent_samples(Duration::from_secs(900), now);

        Self {
            hostname: host.hostname().to_string(),
            address: host.address().to_string(),
            resolved_address: host.resolved_address().to_string(),
            metrics: host.metrics().clone(),
            status: host.status(now),
            sample_count: host.sample_count(),
            recent_samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ProbeError;
    use std::time::Duration;

    fn test_config() -> ProbeConfig {
        ProbeConfig::new(
            Duration::from_secs(1),
            Duration::from_millis(500),
            Duration::from_secs(60),
            60,
        )
    }

    #[test]
    fn new_store_empty() {
        let store = MetricsStore::new(vec![], test_config());
        assert_eq!(store.host_count(), 0);
    }

    #[test]
    fn new_store_with_hosts() {
        let hosts = vec![
            (
                "host1".to_string(),
                "127.0.0.1:80".to_string(),
                "127.0.0.1:80".to_string(),
            ),
            (
                "host2".to_string(),
                "127.0.0.1:81".to_string(),
                "127.0.0.1:81".to_string(),
            ),
        ];
        let store = MetricsStore::new(hosts, test_config());

        assert_eq!(store.host_count(), 2);
        assert!(store.get_host(0).is_some());
        assert!(store.get_host(1).is_some());
        assert!(store.get_host(2).is_none());
    }

    #[test]
    fn handle_probe_result() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(42)));

        let host = store.get_host(0).unwrap();
        assert_eq!(host.sample_count(), 1);
        assert_eq!(host.metrics().success_rate, 100.0);
    }

    #[test]
    fn handle_probe_result_invalid_index() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        // Should not panic with invalid index
        store.handle_probe_result(5, ProbeResult::success(now, Duration::from_millis(42)));

        let host = store.get_host(0).unwrap();
        assert_eq!(host.sample_count(), 0); // No change
    }

    #[test]
    fn update_all_metrics() {
        let hosts = vec![
            (
                "host1".to_string(),
                "127.0.0.1:80".to_string(),
                "127.0.0.1:80".to_string(),
            ),
            (
                "host2".to_string(),
                "127.0.0.1:81".to_string(),
                "127.0.0.1:80".to_string(),
            ),
        ];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(10)));
        store.handle_probe_result(1, ProbeResult::success(now, Duration::from_millis(20)));

        store.update_all_metrics();

        assert_eq!(
            store.get_host(0).unwrap().metrics().avg_latency,
            Some(Duration::from_millis(10))
        );
        assert_eq!(
            store.get_host(1).unwrap().metrics().avg_latency,
            Some(Duration::from_millis(20))
        );
    }

    #[test]
    fn get_overall_metrics() {
        let hosts = vec![
            (
                "host1".to_string(),
                "127.0.0.1:80".to_string(),
                "127.0.0.1:80".to_string(),
            ),
            (
                "host2".to_string(),
                "127.0.0.1:81".to_string(),
                "127.0.0.1:80".to_string(),
            ),
        ];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(10)));
        store.handle_probe_result(1, ProbeResult::failure(now, ProbeError::Timeout));

        let overall = store.get_overall_metrics(now);
        assert_eq!(overall.overall_success_rate, 50.0);
        assert_eq!(overall.total_failures, 1);
    }

    #[test]
    fn snapshot_captures_state() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(42)));

        let snapshot = store.snapshot(now);
        assert_eq!(snapshot.hosts.len(), 1);
        assert_eq!(snapshot.hosts[0].hostname, "host1");
        assert_eq!(snapshot.hosts[0].sample_count, 1);
    }

    #[test]
    fn snapshot_is_independent() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let mut store = MetricsStore::new(hosts, test_config());
        let now = Instant::now();

        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(10)));
        let snapshot1 = store.snapshot(now);

        // Add more data
        store.handle_probe_result(0, ProbeResult::success(now, Duration::from_millis(20)));
        let snapshot2 = store.snapshot(now);

        // Snapshots should differ
        assert_eq!(snapshot1.hosts[0].sample_count, 1);
        assert_eq!(snapshot2.hosts[0].sample_count, 2);
    }

    #[test]
    fn hosts_accessor() {
        let hosts = vec![
            (
                "host1".to_string(),
                "127.0.0.1:80".to_string(),
                "127.0.0.1:80".to_string(),
            ),
            (
                "host2".to_string(),
                "127.0.0.1:81".to_string(),
                "127.0.0.1:80".to_string(),
            ),
        ];
        let store = MetricsStore::new(hosts, test_config());

        let all_hosts = store.hosts();
        assert_eq!(all_hosts.len(), 2);
        assert_eq!(all_hosts[0].hostname(), "host1");
        assert_eq!(all_hosts[1].hostname(), "host2");
    }
}
