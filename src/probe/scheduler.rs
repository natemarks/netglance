//! Probe scheduler for coordinating periodic TCP probes.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::metrics::ProbeResult;
use crate::probe::worker::probe_tcp;

/// Coordinates periodic TCP probes for multiple hosts.
///
/// The scheduler runs in the background and probes all configured hosts
/// at a fixed interval, sending results through a channel.
pub struct ProbeScheduler {
    /// Addresses to probe (host:port format)
    addresses: Vec<String>,
    /// Time between probe rounds
    interval: Duration,
    /// Timeout for each individual probe
    timeout: Duration,
    /// Channel to send probe results
    result_tx: mpsc::UnboundedSender<(usize, ProbeResult)>,
    /// Token for graceful shutdown
    cancellation_token: CancellationToken,
}

impl ProbeScheduler {
    /// Create a new probe scheduler.
    ///
    /// # Arguments
    /// * `addresses` - List of addresses to probe (e.g., ["api.example.com:443"])
    /// * `interval` - Time between probe rounds
    /// * `timeout` - Maximum time for each probe
    /// * `result_tx` - Channel to send results (host_index, ProbeResult)
    /// * `cancellation_token` - Token for graceful shutdown
    pub fn new(
        addresses: Vec<String>,
        interval: Duration,
        timeout: Duration,
        result_tx: mpsc::UnboundedSender<(usize, ProbeResult)>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            addresses,
            interval,
            timeout,
            result_tx,
            cancellation_token,
        }
    }

    /// Run the probe scheduler.
    ///
    /// This method runs indefinitely until the cancellation token is triggered.
    /// It probes all hosts in parallel at each interval tick.
    pub async fn run(self) {
        let addresses = Arc::new(self.addresses);
        let timeout = self.timeout;
        let result_tx = self.result_tx;
        let cancellation_token = self.cancellation_token;

        let mut ticker = interval(self.interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        info!(
            "Starting probe scheduler for {} hosts with interval {:?}",
            addresses.len(),
            self.interval
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    Self::probe_all_hosts(&addresses, timeout, &result_tx).await;
                }
                _ = cancellation_token.cancelled() => {
                    info!("Probe scheduler shutting down");
                    break;
                }
            }
        }
    }

    /// Probe all hosts concurrently.
    async fn probe_all_hosts(
        addresses: &Arc<Vec<String>>,
        timeout: Duration,
        result_tx: &mpsc::UnboundedSender<(usize, ProbeResult)>,
    ) {
        let mut tasks = Vec::new();

        for (index, address) in addresses.iter().enumerate() {
            let address = address.clone();
            let result_tx = result_tx.clone();

            // Spawn a task for each probe
            let task = tokio::spawn(async move {
                let result = probe_tcp(&address, timeout).await;

                // Send result through channel
                if let Err(e) = result_tx.send((index, result)) {
                    error!("Failed to send probe result for host {}: {}", index, e);
                }
            });

            tasks.push(task);
        }

        // Wait for all probes to complete
        for (index, task) in tasks.into_iter().enumerate() {
            if let Err(e) = task.await {
                error!("Probe task {} panicked: {}", index, e);
            }
        }

        debug!("Completed probe round for {} hosts", addresses.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use tokio::time::{sleep, timeout as tokio_timeout};

    #[tokio::test(flavor = "multi_thread")]
    async fn scheduler_probes_hosts() {
        // Start a local TCP server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();

        // Accept connections in background
        let listener_task = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        if result.is_ok() {
                            // Connection accepted
                        }
                    }
                    _ = sleep(Duration::from_millis(10)) => {
                        // Keep accepting
                    }
                }
            }
        });

        // Create channel and scheduler
        let (tx, mut rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let scheduler = ProbeScheduler::new(
            vec![addr.to_string()],
            Duration::from_millis(50), // Fast interval for testing
            Duration::from_millis(500),
            tx,
            cancel.clone(),
        );

        // Run scheduler in background
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        // Wait for at least one probe result
        let result = tokio_timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("Should receive result within timeout")
            .expect("Channel should not be closed");

        assert_eq!(result.0, 0); // First host
        assert!(result.1.is_success(), "Probe should succeed");

        // Cancel scheduler
        cancel.cancel();
        let _ = tokio_timeout(Duration::from_secs(2), scheduler_handle).await;

        // Clean up listener task
        listener_task.abort();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scheduler_probes_multiple_hosts() {
        // Start two local TCP servers
        let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr1 = listener1.local_addr().unwrap();
        listener1.set_nonblocking(true).unwrap();

        let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr2 = listener2.local_addr().unwrap();
        listener2.set_nonblocking(true).unwrap();

        // Accept connections in background
        let task1 = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::from_std(listener1).unwrap();
            loop {
                let _ = listener.accept().await;
            }
        });
        let task2 = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::from_std(listener2).unwrap();
            loop {
                let _ = listener.accept().await;
            }
        });

        // Create scheduler
        let (tx, mut rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let scheduler = ProbeScheduler::new(
            vec![addr1.to_string(), addr2.to_string()],
            Duration::from_millis(50),
            Duration::from_millis(500),
            tx,
            cancel.clone(),
        );

        // Run scheduler
        tokio::spawn(async move {
            scheduler.run().await;
        });

        // Collect results
        let mut received = Vec::new();
        for _ in 0..2 {
            if let Ok(Some(result)) = tokio_timeout(Duration::from_secs(5), rx.recv()).await {
                received.push(result);
            }
        }

        // Should have received results from both hosts
        assert_eq!(received.len(), 2, "Should receive results from both hosts");
        assert!(received.iter().any(|(idx, _)| *idx == 0));
        assert!(received.iter().any(|(idx, _)| *idx == 1));

        cancel.cancel();
        task1.abort();
        task2.abort();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scheduler_handles_failures() {
        // Use a port that won't be listening
        let (tx, mut rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let scheduler = ProbeScheduler::new(
            vec!["127.0.0.1:1".to_string()], // Port 1 should refuse
            Duration::from_millis(50),
            Duration::from_millis(500),
            tx,
            cancel.clone(),
        );

        tokio::spawn(async move {
            scheduler.run().await;
        });

        // Should receive a failure result
        let result = tokio_timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("Should receive result")
            .expect("Channel open");

        assert_eq!(result.0, 0);
        assert!(result.1.is_failure(), "Probe should fail for port 1");

        cancel.cancel();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scheduler_cancellation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let scheduler = ProbeScheduler::new(
            vec!["127.0.0.1:12345".to_string()],
            Duration::from_secs(10), // Long interval
            Duration::from_millis(500),
            tx,
            cancel.clone(),
        );

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        // Cancel immediately
        sleep(Duration::from_millis(50)).await;
        cancel.cancel();

        // Scheduler should exit quickly
        let result = tokio_timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Scheduler should exit on cancellation");
    }
}
