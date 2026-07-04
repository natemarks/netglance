//! Host health status classification.

use std::fmt;

/// Health status of a monitored host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostStatus {
    /// Waiting for first successful probe
    Waiting,
    /// All probes successful, host is healthy
    Healthy,
    /// Some failures, but mostly working
    Degraded,
    /// Multiple consecutive failures or high failure rate
    Down,
    /// No recent probe data available
    Stale,
    /// Never had a successful probe (critical state)
    NeverSucceeded,
}

impl HostStatus {
    /// Determine status based on probe statistics.
    ///
    /// # Classification Rules
    /// - **Waiting**: No probes yet (total = 0)
    /// - **NeverSucceeded**: No successful probes after initial attempts (total > 5)
    /// - **Stale**: No probes in the last 30 seconds
    /// - **Down**: 3+ consecutive failures OR failure rate >= 40%
    /// - **Degraded**: 1-2 consecutive failures OR success rate 90-95%
    /// - **Healthy**: Everything else
    pub fn classify(
        success_rate: f64,
        consecutive_failures: usize,
        has_recent_data: bool,
        total_probes: usize,
    ) -> Self {
        // Waiting: no probes yet
        if total_probes == 0 {
            return HostStatus::Waiting;
        }

        // Critical: never succeeded after initial probe attempts
        if success_rate == 0.0 && total_probes > 5 {
            return HostStatus::NeverSucceeded;
        }

        if !has_recent_data {
            return HostStatus::Stale;
        }

        if consecutive_failures >= 3 || success_rate < 60.0 {
            return HostStatus::Down;
        }

        if consecutive_failures >= 1 || success_rate < 95.0 {
            return HostStatus::Degraded;
        }

        HostStatus::Healthy
    }

    /// Get a text indicator for display.
    pub fn indicator(&self) -> &'static str {
        match self {
            HostStatus::Waiting => "⧗ WAITING",
            HostStatus::Healthy => "● HEALTHY",
            HostStatus::Degraded => "⚠ DEGRADED",
            HostStatus::Down => "✗ DOWN",
            HostStatus::Stale => "○ STALE",
            HostStatus::NeverSucceeded => "💀 NEVER SUCCEEDED",
        }
    }

    /// Check if status indicates a problem.
    #[allow(dead_code)]
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            HostStatus::Degraded | HostStatus::Down | HostStatus::NeverSucceeded
        )
    }

    /// Check if status indicates waiting for first probe.
    #[allow(dead_code)]
    pub fn is_waiting(&self) -> bool {
        matches!(self, HostStatus::Waiting)
    }

    /// Get the color for UI rendering.
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            HostStatus::Waiting => Color::Cyan,
            HostStatus::Healthy => Color::Green,
            HostStatus::Degraded => Color::Yellow,
            HostStatus::Down | HostStatus::NeverSucceeded => Color::Red,
            HostStatus::Stale => Color::DarkGray,
        }
    }

    /// Get short indicator for table display.
    pub fn short_indicator(&self) -> &'static str {
        match self {
            HostStatus::Waiting => "⧗ WAIT",
            HostStatus::Healthy => "✓ OK",
            HostStatus::Degraded => "! WARN",
            HostStatus::Down => "✗ DOWN",
            HostStatus::Stale => "- STALE",
            HostStatus::NeverSucceeded => "💀 NEVER",
        }
    }
}

impl fmt::Display for HostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.indicator())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_never_succeeded() {
        // Never succeeded after initial probes
        let status = HostStatus::classify(0.0, 10, true, 10);
        assert_eq!(status, HostStatus::NeverSucceeded);

        // Not enough probes yet - should be Down, not NeverSucceeded
        let status = HostStatus::classify(0.0, 3, true, 3);
        assert_eq!(status, HostStatus::Down);
    }

    #[test]
    fn classify_healthy() {
        let status = HostStatus::classify(100.0, 0, true, 100);
        assert_eq!(status, HostStatus::Healthy);

        let status = HostStatus::classify(98.0, 0, true, 100);
        assert_eq!(status, HostStatus::Healthy);
    }

    #[test]
    fn classify_degraded() {
        // Recent failure
        let status = HostStatus::classify(100.0, 1, true, 100);
        assert_eq!(status, HostStatus::Degraded);

        // Low success rate
        let status = HostStatus::classify(92.0, 0, true, 100);
        assert_eq!(status, HostStatus::Degraded);

        // Two consecutive failures
        let status = HostStatus::classify(90.0, 2, true, 100);
        assert_eq!(status, HostStatus::Degraded);
    }

    #[test]
    fn classify_down() {
        // Three consecutive failures
        let status = HostStatus::classify(70.0, 3, true, 100);
        assert_eq!(status, HostStatus::Down);

        // Very low success rate
        let status = HostStatus::classify(50.0, 0, true, 100);
        assert_eq!(status, HostStatus::Down);

        // Many consecutive failures
        let status = HostStatus::classify(20.0, 10, true, 100);
        assert_eq!(status, HostStatus::Down);
    }

    #[test]
    fn classify_stale() {
        // No recent data
        let status = HostStatus::classify(100.0, 0, false, 100);
        assert_eq!(status, HostStatus::Stale);

        // Even with perfect stats, no data = stale
        let status = HostStatus::classify(100.0, 0, false, 100);
        assert_eq!(status, HostStatus::Stale);
    }

    #[test]
    fn classify_waiting() {
        // No probes yet
        let status = HostStatus::classify(0.0, 0, false, 0);
        assert_eq!(status, HostStatus::Waiting);
    }

    #[test]
    fn indicator_strings() {
        assert_eq!(HostStatus::Waiting.indicator(), "⧗ WAITING");
        assert_eq!(HostStatus::Healthy.indicator(), "● HEALTHY");
        assert_eq!(HostStatus::Degraded.indicator(), "⚠ DEGRADED");
        assert_eq!(HostStatus::Down.indicator(), "✗ DOWN");
        assert_eq!(HostStatus::Stale.indicator(), "○ STALE");
        assert_eq!(HostStatus::NeverSucceeded.indicator(), "💀 NEVER SUCCEEDED");
    }

    #[test]
    fn is_problem_detection() {
        assert!(!HostStatus::Waiting.is_problem());
        assert!(!HostStatus::Healthy.is_problem());
        assert!(HostStatus::Degraded.is_problem());
        assert!(HostStatus::Down.is_problem());
        assert!(!HostStatus::Stale.is_problem());
        assert!(HostStatus::NeverSucceeded.is_problem());
    }

    #[test]
    fn display_format() {
        assert_eq!(HostStatus::Waiting.to_string(), "⧗ WAITING");
        assert_eq!(HostStatus::Healthy.to_string(), "● HEALTHY");
        assert_eq!(HostStatus::Degraded.to_string(), "⚠ DEGRADED");
        assert_eq!(HostStatus::Down.to_string(), "✗ DOWN");
        assert_eq!(HostStatus::Stale.to_string(), "○ STALE");
        assert_eq!(HostStatus::NeverSucceeded.to_string(), "💀 NEVER SUCCEEDED");
    }
}
