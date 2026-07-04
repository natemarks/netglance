//! Overall metrics aggregation across all hosts.

use std::time::{Duration, Instant};

use super::{HostState, HostStatus};

/// Aggregated metrics across all monitored hosts.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OverallMetrics {
    /// Average latency across all successful probes
    pub avg_latency: Option<Duration>,
    /// 95th percentile latency across all hosts
    pub p95_latency: Option<Duration>,
    /// Overall success rate as percentage (0.0 - 100.0)
    pub overall_success_rate: f64,
    /// Total number of failures across all hosts
    pub total_failures: usize,
    /// Approximate failure rate per minute
    pub failure_rate_per_minute: f64,
    /// Number of healthy hosts
    pub healthy_count: usize,
    /// Number of degraded hosts
    pub degraded_count: usize,
    /// Number of down hosts
    pub down_count: usize,
    /// Number of stale hosts (no recent data)
    pub stale_count: usize,
}

impl OverallMetrics {
    /// Calculate overall metrics from all host states.
    ///
    /// # Arguments
    /// * `hosts` - All monitored hosts
    /// * `now` - Current time for status determination
    #[must_use]
    pub fn calculate(hosts: &[HostState], now: Instant) -> Self {
        if hosts.is_empty() {
            return Self::empty();
        }

        let mut all_latencies = Vec::new();
        let mut total_probes = 0;
        let mut total_successes = 0;
        let mut total_failures = 0;

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut down_count = 0;
        let mut stale_count = 0;

        // Aggregate data from all hosts
        for host in hosts {
            let metrics = host.metrics();

            // Count status
            match host.status(now) {
                HostStatus::Waiting => stale_count += 1, // Count as stale for now
                HostStatus::Healthy => healthy_count += 1,
                HostStatus::Degraded => degraded_count += 1,
                HostStatus::Down | HostStatus::NeverSucceeded => down_count += 1,
                HostStatus::Stale => stale_count += 1,
            }

            // Collect latencies from successful probes
            if let Some(avg) = metrics.avg_latency {
                all_latencies.push(avg);
            }

            // Count successes and failures
            let sample_count = host.sample_count();
            total_probes += sample_count;
            total_failures += metrics.failure_count;
            total_successes += sample_count - metrics.failure_count;
        }

        // Calculate aggregate statistics
        let overall_success_rate = if total_probes > 0 {
            (total_successes as f64 / total_probes as f64) * 100.0
        } else {
            0.0
        };

        let avg_latency = if !all_latencies.is_empty() {
            let sum: Duration = all_latencies.iter().sum();
            Some(sum / all_latencies.len() as u32)
        } else {
            None
        };

        // P95 calculation across all host averages
        let p95_latency = calculate_percentile(&all_latencies, 95.0);

        // Estimate failure rate per minute
        // Assuming typical 1-second interval, this is failures per 60 probes
        let failure_rate_per_minute = if total_probes > 0 {
            (total_failures as f64 / total_probes as f64) * 60.0
        } else {
            0.0
        };

        Self {
            avg_latency,
            p95_latency,
            overall_success_rate,
            total_failures,
            failure_rate_per_minute,
            healthy_count,
            degraded_count,
            down_count,
            stale_count,
        }
    }

    /// Create empty overall metrics.
    pub fn empty() -> Self {
        Self {
            avg_latency: None,
            p95_latency: None,
            overall_success_rate: 0.0,
            total_failures: 0,
            failure_rate_per_minute: 0.0,
            healthy_count: 0,
            degraded_count: 0,
            down_count: 0,
            stale_count: 0,
        }
    }
}

/// Calculate percentile from durations.
fn calculate_percentile(values: &[Duration], percentile: f64) -> Option<Duration> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort();

    let index = (percentile / 100.0) * (sorted.len() - 1) as f64;
    let lower_index = index.floor() as usize;
    let upper_index = index.ceil() as usize;

    if lower_index == upper_index {
        Some(sorted[lower_index])
    } else {
        let lower = sorted[lower_index];
        let upper = sorted[upper_index];
        let fraction = index - lower_index as f64;
        let interpolated = lower + (upper - lower).mul_f64(fraction);
        Some(interpolated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{ProbeError, ProbeResult};

    #[test]
    fn empty_overall_metrics() {
        let hosts: Vec<HostState> = vec![];
        let now = Instant::now();
        let metrics = OverallMetrics::calculate(&hosts, now);

        assert_eq!(metrics.avg_latency, None);
        assert_eq!(metrics.overall_success_rate, 0.0);
        assert_eq!(metrics.total_failures, 0);
        assert_eq!(metrics.healthy_count, 0);
    }

    #[test]
    fn single_host_all_healthy() {
        let mut host = HostState::new(
            "test".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        );
        let now = Instant::now();

        host.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));
        host.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));

        let hosts = vec![host];
        let metrics = OverallMetrics::calculate(&hosts, now);

        assert_eq!(metrics.overall_success_rate, 100.0);
        assert_eq!(metrics.total_failures, 0);
        assert_eq!(metrics.healthy_count, 1);
        assert_eq!(metrics.down_count, 0);
    }

    #[test]
    fn multiple_hosts_mixed_status() {
        let now = Instant::now();

        // Healthy host (need more samples to reach 95% threshold)
        let mut host1 = HostState::new(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        );
        for _ in 0..10 {
            host1.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));
        }

        // Degraded host (90-95% success rate, or 1-2 consecutive failures)
        let mut host2 = HostState::new(
            "host2".to_string(),
            "127.0.0.1:81".to_string(),
            "127.0.0.1:81".to_string(),
        );
        for _ in 0..9 {
            host2.add_probe_result(ProbeResult::success(now, Duration::from_millis(15)));
        }
        host2.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout)); // 90% success rate

        // Down host
        let mut host3 = HostState::new(
            "host3".to_string(),
            "127.0.0.1:82".to_string(),
            "127.0.0.1:82".to_string(),
        );
        host3.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));
        host3.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));
        host3.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));

        let host_list = vec![host1, host2, host3];
        let metrics = OverallMetrics::calculate(&host_list, now);

        assert_eq!(metrics.healthy_count, 1, "Should have 1 healthy host");
        assert_eq!(metrics.degraded_count, 1, "Should have 1 degraded host");
        assert_eq!(metrics.down_count, 1, "Should have 1 down host");
        assert_eq!(metrics.total_failures, 4); // 0 + 1 + 3
        assert!(metrics.overall_success_rate > 60.0); // Overall should be decent
    }

    #[test]
    fn success_rate_calculation() {
        let now = Instant::now();

        let mut host = HostState::new(
            "test".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        );
        host.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));
        host.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));
        host.add_probe_result(ProbeResult::success(now, Duration::from_millis(30)));
        host.add_probe_result(ProbeResult::failure(now, ProbeError::Timeout));

        let hosts = vec![host];
        let metrics = OverallMetrics::calculate(&hosts, now);

        assert_eq!(metrics.overall_success_rate, 75.0); // 3 out of 4
        assert_eq!(metrics.total_failures, 1);
    }

    #[test]
    fn latency_aggregation() {
        let now = Instant::now();

        let mut host1 = HostState::new(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        );
        host1.add_probe_result(ProbeResult::success(now, Duration::from_millis(10)));

        let mut host2 = HostState::new(
            "host2".to_string(),
            "127.0.0.1:81".to_string(),
            "127.0.0.1:81".to_string(),
        );
        host2.add_probe_result(ProbeResult::success(now, Duration::from_millis(20)));

        let host_list = vec![host1, host2];
        let metrics = OverallMetrics::calculate(&host_list, now);

        // Average of averages: (10 + 20) / 2 = 15
        assert_eq!(metrics.avg_latency, Some(Duration::from_millis(15)));
    }

    #[test]
    fn stale_host_detection() {
        let now = Instant::now();
        let old = now - Duration::from_secs(60);

        let mut host = HostState::new(
            "test".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        );
        host.add_probe_result(ProbeResult::success(old, Duration::from_millis(10)));

        let hosts = vec![host];
        let metrics = OverallMetrics::calculate(&hosts, now);

        assert_eq!(metrics.stale_count, 1);
        assert_eq!(metrics.healthy_count, 0);
    }
}
