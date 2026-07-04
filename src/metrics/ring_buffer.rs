//! Ring buffer implementation for storing probe results with time-based expiration.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::ProbeResult;

/// Rolling window of probe results with automatic expiration.
///
/// Maintains a fixed time window of probe results, automatically removing
/// entries that are older than the configured window duration.
#[derive(Debug)]
pub struct RollingWindow {
    /// Circular buffer of probe results
    data: VecDeque<ProbeResult>,
    /// How far back in time to keep samples
    window_duration: Duration,
    /// Maximum number of samples to prevent unbounded growth
    max_samples: usize,
}

impl RollingWindow {
    /// Create a new rolling window.
    ///
    /// # Arguments
    /// * `window_duration` - How long to keep samples (e.g., 10 minutes)
    /// * `max_samples` - Maximum number of samples to store
    pub fn new(window_duration: Duration, max_samples: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_samples.min(1024)),
            window_duration,
            max_samples,
        }
    }

    /// Add a new probe result to the window.
    ///
    /// This will automatically expire old entries and enforce the max sample limit.
    pub fn push(&mut self, result: ProbeResult) {
        // Remove old entries based on time window
        self.expire_old(result.timestamp);

        // Add new result
        self.data.push_back(result);

        // Enforce max samples by removing oldest if needed
        while self.data.len() > self.max_samples {
            self.data.pop_front();
        }
    }

    /// Remove samples older than the window duration.
    ///
    /// # Arguments
    /// * `now` - Current time reference
    pub fn expire_old(&mut self, now: Instant) {
        // Calculate the cutoff time: samples older than this should be removed
        let cutoff = now - self.window_duration;

        // Remove all entries older than the cutoff
        while let Some(oldest) = self.data.front() {
            if oldest.timestamp < cutoff {
                self.data.pop_front();
            } else {
                break;
            }
        }
    }

    /// Iterate over all probe results in the window.
    pub fn iter(&self) -> impl Iterator<Item = &ProbeResult> {
        self.data.iter()
    }

    /// Get the number of samples in the window.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the window is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the window duration configuration.
    #[allow(dead_code)]
    pub fn window_duration(&self) -> Duration {
        self.window_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ProbeError;

    #[test]
    fn new_window_is_empty() {
        let window = RollingWindow::new(Duration::from_secs(600), 100);
        assert_eq!(window.len(), 0);
        assert!(window.is_empty());
    }

    #[test]
    fn push_adds_samples() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::success(now, Duration::from_millis(10)));
        assert_eq!(window.len(), 1);

        window.push(ProbeResult::success(now, Duration::from_millis(20)));
        assert_eq!(window.len(), 2);
    }

    #[test]
    fn expire_old_removes_old_samples() {
        let mut window = RollingWindow::new(Duration::from_secs(10), 100);
        let base_time = Instant::now();

        // Add samples at sequential times (so push doesn't auto-expire)
        let t1 = base_time;
        let t2 = base_time + Duration::from_millis(100);
        let t3 = base_time + Duration::from_millis(200);

        window.push(ProbeResult::success(t1, Duration::from_millis(10)));
        window.push(ProbeResult::success(t2, Duration::from_millis(20)));
        window.push(ProbeResult::success(t3, Duration::from_millis(30)));

        assert_eq!(window.len(), 3);

        // Expire old samples (check from 15 seconds in the future)
        // This should remove samples older than (now - 10s) = 5s from base_time
        let now = base_time + Duration::from_secs(15);
        window.expire_old(now);

        // All samples should be removed (they're all older than 10 seconds)
        assert_eq!(window.len(), 0);
    }

    #[test]
    fn max_samples_enforced() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 5);
        let now = Instant::now();

        // Add 10 samples
        for i in 0..10 {
            window.push(ProbeResult::success(
                now + Duration::from_millis(i * 100),
                Duration::from_millis(10),
            ));
        }

        // Should only keep 5 (the most recent)
        assert_eq!(window.len(), 5);
    }

    #[test]
    fn iter_returns_all_samples() {
        let mut window = RollingWindow::new(Duration::from_secs(600), 100);
        let now = Instant::now();

        window.push(ProbeResult::success(now, Duration::from_millis(10)));
        window.push(ProbeResult::success(now, Duration::from_millis(20)));
        window.push(ProbeResult::failure(now, ProbeError::Timeout));

        let samples: Vec<_> = window.iter().collect();
        assert_eq!(samples.len(), 3);
        assert!(samples[0].is_success());
        assert!(samples[1].is_success());
        assert!(samples[2].is_failure());
    }

    #[test]
    fn push_with_auto_expire() {
        let mut window = RollingWindow::new(Duration::from_secs(10), 100);
        let now = Instant::now();

        // Add old sample
        window.push(ProbeResult::success(
            now - Duration::from_secs(20),
            Duration::from_millis(10),
        ));
        assert_eq!(window.len(), 1);

        // Push new sample which should trigger expiration
        window.push(ProbeResult::success(now, Duration::from_millis(20)));

        // Old sample should be gone
        assert_eq!(window.len(), 1);
    }
}
