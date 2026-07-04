//! Statistical calculations for host metrics.

use std::time::{Duration, Instant};

use super::RollingWindow;

/// Calculated metrics for a monitored host.
#[derive(Debug, Clone)]
pub struct HostMetrics {
    /// Average latency of successful probes
    pub avg_latency: Option<Duration>,
    /// Minimum latency observed
    pub min_latency: Option<Duration>,
    /// Maximum latency observed
    pub max_latency: Option<Duration>,
    /// 95th percentile latency
    pub p95_latency: Option<Duration>,
    /// Success rate as a percentage (0.0 - 100.0)
    pub success_rate: f64,
    /// Total number of failed probes
    pub failure_count: usize,
    /// Number of consecutive failures (most recent)
    pub consecutive_failures: usize,
    /// Total number of probes attempted (success + failure)
    pub total_probes: usize,
    /// When the last successful probe occurred
    pub last_success: Option<Instant>,
    /// When the most recent probe occurred (success or failure)
    pub last_check: Option<Instant>,
}

impl HostMetrics {
    /// Calculate metrics from a rolling window of probe results.
    pub fn from_window(window: &RollingWindow) -> Self {
        if window.is_empty() {
            return Self::empty();
        }

        let samples: Vec<_> = window.iter().collect();
        let total_count = samples.len();

        // Collect successful probes
        let successes: Vec<Duration> = samples.iter().filter_map(|r| r.latency).collect();

        let failure_count = total_count - successes.len();
        let success_rate = if total_count > 0 {
            (successes.len() as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        // Calculate latency statistics
        let avg_latency = if !successes.is_empty() {
            let sum: Duration = successes.iter().sum();
            Some(sum / successes.len() as u32)
        } else {
            None
        };

        let min_latency = successes.iter().min().copied();
        let max_latency = successes.iter().max().copied();
        let p95_latency = calculate_percentile(&successes, 95.0);

        // Count consecutive failures from the end
        let consecutive_failures = samples.iter().rev().take_while(|r| r.is_failure()).count();

        // Find last successful probe
        let last_success = samples
            .iter()
            .rev()
            .find(|r| r.is_success())
            .map(|r| r.timestamp);

        // Most recent probe timestamp
        let last_check = samples.last().map(|r| r.timestamp);

        Self {
            avg_latency,
            min_latency,
            max_latency,
            p95_latency,
            success_rate,
            failure_count,
            consecutive_failures,
            total_probes: total_count,
            last_success,
            last_check,
        }
    }

    /// Create empty metrics (no data).
    pub fn empty() -> Self {
        Self {
            avg_latency: None,
            min_latency: None,
            max_latency: None,
            p95_latency: None,
            success_rate: 0.0,
            failure_count: 0,
            consecutive_failures: 0,
            total_probes: 0,
            last_success: None,
            last_check: None,
        }
    }

    /// Check if there is recent data (within the last 30 seconds).
    pub fn has_recent_data(&self, now: Instant) -> bool {
        if let Some(last_check) = self.last_check {
            now.duration_since(last_check) < Duration::from_secs(30)
        } else {
            false
        }
    }
}

/// Calculate a percentile from a list of durations.
///
/// Uses linear interpolation between data points.
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
        // Linear interpolation
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
    fn empty_metrics() {
        let metrics = HostMetrics::empty();
        assert_eq!(metrics.avg_latency, None);
        assert_eq!(metrics.min_latency, None);
        assert_eq!(metrics.max_latency, None);
        assert_eq!(metrics.p95_latency, None);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.failure_count, 0);
        assert_eq!(metrics.consecutive_failures, 0);
    }

    #[test]
    fn metrics_from_empty_window() {
        let window = RollingWindow::new(Duration::from_secs(600), 100);
        let metrics = HostMetrics::from_window(&window);

        assert_eq!(metrics.avg_latency, None);
        assert_eq!(metrics.success_rate, 0.0);
    }

    #[test]
    fn metrics_all_successes() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::success(now, Duration::from_millis(10)));
        window.push(ProbeResult::success(now, Duration::from_millis(20)));
        window.push(ProbeResult::success(now, Duration::from_millis(30)));

        let metrics = HostMetrics::from_window(&window);

        assert_eq!(metrics.success_rate, 100.0);
        assert_eq!(metrics.failure_count, 0);
        assert_eq!(metrics.consecutive_failures, 0);
        assert_eq!(metrics.min_latency, Some(Duration::from_millis(10)));
        assert_eq!(metrics.max_latency, Some(Duration::from_millis(30)));
        assert_eq!(metrics.avg_latency, Some(Duration::from_millis(20)));
    }

    #[test]
    fn metrics_all_failures() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::failure(now, ProbeError::Timeout));
        window.push(ProbeResult::failure(now, ProbeError::ConnectionRefused));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));

        let metrics = HostMetrics::from_window(&window);

        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.failure_count, 3);
        assert_eq!(metrics.consecutive_failures, 3);
        assert_eq!(metrics.avg_latency, None);
        assert_eq!(metrics.last_success, None);
    }

    #[test]
    fn metrics_mixed_results() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::success(now, Duration::from_millis(10)));
        window.push(ProbeResult::success(now, Duration::from_millis(20)));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));
        window.push(ProbeResult::success(now, Duration::from_millis(30)));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));

        let metrics = HostMetrics::from_window(&window);

        assert_eq!(metrics.success_rate, 60.0); // 3 out of 5
        assert_eq!(metrics.failure_count, 2);
        assert_eq!(metrics.consecutive_failures, 1); // Only last one
        assert_eq!(metrics.avg_latency, Some(Duration::from_millis(20)));
    }

    #[test]
    fn consecutive_failures_count() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::success(now, Duration::from_millis(10)));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));

        let metrics = HostMetrics::from_window(&window);
        assert_eq!(metrics.consecutive_failures, 3);
    }

    #[test]
    fn percentile_calculation() {
        let values = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];

        let p50 = calculate_percentile(&values, 50.0);
        assert_eq!(p50, Some(Duration::from_millis(30)));

        let p95 = calculate_percentile(&values, 95.0);
        assert!(p95.unwrap() >= Duration::from_millis(45));
    }

    #[test]
    fn percentile_single_value() {
        let values = vec![Duration::from_millis(42)];
        let p95 = calculate_percentile(&values, 95.0);
        assert_eq!(p95, Some(Duration::from_millis(42)));
    }

    #[test]
    fn percentile_empty() {
        let values: Vec<Duration> = vec![];
        let p95 = calculate_percentile(&values, 95.0);
        assert_eq!(p95, None);
    }

    #[test]
    fn has_recent_data() {
        let now = Instant::now();
        let mut metrics = HostMetrics::empty();

        // No data
        assert!(!metrics.has_recent_data(now));

        // Recent data (5 seconds ago)
        metrics.last_check = Some(now - Duration::from_secs(5));
        assert!(metrics.has_recent_data(now));

        // Stale data (60 seconds ago)
        metrics.last_check = Some(now - Duration::from_secs(60));
        assert!(!metrics.has_recent_data(now));
    }
}
