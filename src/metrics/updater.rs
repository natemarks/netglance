//! Background metrics updater task.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use super::{store::MetricsStore, ProbeResult};

/// Background task that receives probe results and updates the metrics store.
pub struct MetricsUpdater {
    /// Shared metrics store
    store: Arc<RwLock<MetricsStore>>,
    /// Channel to receive probe results
    result_rx: mpsc::UnboundedReceiver<(usize, ProbeResult)>,
    /// How often to expire old samples
    update_interval: Duration,
    /// Cancellation token for shutdown
    cancellation_token: CancellationToken,
}

impl MetricsUpdater {
    /// Create a new metrics updater.
    ///
    /// # Arguments
    /// * `store` - Shared metrics store
    /// * `result_rx` - Channel receiving probe results
    /// * `update_interval` - How often to update/expire metrics (e.g., every 10 seconds)
    /// * `cancellation_token` - Token for graceful shutdown
    pub fn new(
        store: Arc<RwLock<MetricsStore>>,
        result_rx: mpsc::UnboundedReceiver<(usize, ProbeResult)>,
        update_interval: Duration,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            store,
            result_rx,
            update_interval,
            cancellation_token,
        }
    }

    /// Run the metrics updater loop.
    ///
    /// This method runs indefinitely until:
    /// - The cancellation token is triggered, or
    /// - The probe result channel is closed
    pub async fn run(mut self) {
        let mut ticker = interval(self.update_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        info!("Starting metrics updater");

        loop {
            tokio::select! {
                // Receive probe results
                result = self.result_rx.recv() => {
                    match result {
                        Some((host_idx, probe_result)) => {
                            self.handle_probe_result(host_idx, probe_result).await;
                        }
                        None => {
                            info!("Probe result channel closed, shutting down");
                            break;
                        }
                    }
                }

                // Periodic update to expire old samples
                _ = ticker.tick() => {
                    self.update_metrics().await;
                }

                // Graceful shutdown
                _ = self.cancellation_token.cancelled() => {
                    info!("Metrics updater shutting down");
                    break;
                }
            }
        }

        // Process any remaining messages before exit
        self.drain_remaining().await;
    }

    /// Handle a single probe result.
    async fn handle_probe_result(&self, host_idx: usize, result: ProbeResult) {
        let mut store = self.store.write().await;
        store.handle_probe_result(host_idx, result);
        // Note: We don't call update_all_metrics here for performance
        // It will be called periodically by the ticker
    }

    /// Update all metrics (expire old samples, recalculate statistics).
    async fn update_metrics(&self) {
        let mut store = self.store.write().await;
        store.update_all_metrics();
        debug!("Updated metrics for {} hosts", store.host_count());
    }

    /// Drain any remaining probe results before shutdown.
    async fn drain_remaining(&mut self) {
        let mut count = 0;
        while let Ok((host_idx, result)) = self.result_rx.try_recv() {
            self.handle_probe_result(host_idx, result).await;
            count += 1;
        }

        if count > 0 {
            info!("Processed {} remaining probe results", count);
            self.update_metrics().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProbeConfig;
    use crate::metrics::store::MetricsStore;
    use std::time::Instant;
    use tokio::time::{sleep, timeout};

    fn test_config() -> ProbeConfig {
        ProbeConfig::new(
            Duration::from_secs(1),
            Duration::from_millis(500),
            Duration::from_secs(60),
            60,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updater_processes_results() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let store = Arc::new(RwLock::new(MetricsStore::new(hosts, test_config())));

        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let updater = MetricsUpdater::new(
            store.clone(),
            rx,
            Duration::from_millis(100),
            cancel.clone(),
        );

        // Run updater in background
        tokio::spawn(async move {
            updater.run().await;
        });

        // Send probe results
        let now = Instant::now();
        tx.send((0, ProbeResult::success(now, Duration::from_millis(42))))
            .unwrap();

        // Wait for processing
        sleep(Duration::from_millis(50)).await;

        // Check store was updated
        let store_read = store.read().await;
        assert_eq!(store_read.get_host(0).unwrap().sample_count(), 1);

        cancel.cancel();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updater_periodic_updates() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let store = Arc::new(RwLock::new(MetricsStore::new(hosts, test_config())));

        let (_tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let updater = MetricsUpdater::new(
            store.clone(),
            rx,
            Duration::from_millis(50), // Fast update for testing
            cancel.clone(),
        );

        tokio::spawn(async move {
            updater.run().await;
        });

        // Wait for a few update cycles
        sleep(Duration::from_millis(150)).await;

        cancel.cancel();
        // Test passes if no panic occurred
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updater_shutdown_on_channel_close() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let store = Arc::new(RwLock::new(MetricsStore::new(hosts, test_config())));

        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let updater = MetricsUpdater::new(store, rx, Duration::from_millis(100), cancel);

        let handle = tokio::spawn(async move {
            updater.run().await;
        });

        // Close channel
        drop(tx);

        // Updater should exit
        let result = timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Updater should exit when channel closes");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updater_graceful_cancellation() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let store = Arc::new(RwLock::new(MetricsStore::new(hosts, test_config())));

        let (_tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let updater = MetricsUpdater::new(
            store,
            rx,
            Duration::from_secs(10), // Long interval
            cancel.clone(),
        );

        let handle = tokio::spawn(async move {
            updater.run().await;
        });

        // Cancel immediately
        sleep(Duration::from_millis(50)).await;
        cancel.cancel();

        // Should exit quickly
        let result = timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Updater should exit on cancellation");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updater_drains_on_shutdown() {
        let hosts = vec![(
            "host1".to_string(),
            "127.0.0.1:80".to_string(),
            "127.0.0.1:80".to_string(),
        )];
        let store = Arc::new(RwLock::new(MetricsStore::new(hosts, test_config())));

        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let updater = MetricsUpdater::new(
            store.clone(),
            rx,
            Duration::from_millis(100),
            cancel.clone(),
        );

        tokio::spawn(async move {
            updater.run().await;
        });

        // Send some results
        let now = Instant::now();
        for _ in 0..5 {
            tx.send((0, ProbeResult::success(now, Duration::from_millis(10))))
                .unwrap();
        }

        // Cancel and wait for drain
        cancel.cancel();
        sleep(Duration::from_millis(200)).await;

        // All results should be processed
        let store_read = store.read().await;
        assert_eq!(store_read.get_host(0).unwrap().sample_count(), 5);
    }
}
