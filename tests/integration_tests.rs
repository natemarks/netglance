//! Integration tests for complete workflows.
//!
//! These tests wire up multiple components together and verify they work as a system.

mod helpers;

use helpers::TestServer;
use netglance::{MetricsStore, MetricsUpdater, ProbeConfig, ProbeScheduler};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

/// Test end-to-end monitoring with a local TCP server.
///
/// Verifies: ProbeScheduler → Channel → MetricsUpdater → MetricsStore
#[tokio::test(flavor = "multi_thread")]
async fn test_end_to_end_monitoring() {
    // Set up test TCP server (auto-cleanup via RAII)
    let server = TestServer::start();
    let addr = server.addr().to_string();

    // Create configuration with fast timing for testing
    let config = ProbeConfig::new(
        Duration::from_millis(100), // Fast probes
        Duration::from_millis(50),  // Quick timeout
        Duration::from_secs(60),    // 1 minute window
        100,                        // Max 100 samples
    );

    // Create metrics store with one test host
    let hosts = vec![("Test Host".to_string(), addr.clone(), addr.clone())];
    let store = Arc::new(RwLock::new(MetricsStore::new(hosts, config.clone())));

    // Create channel for probe results
    let (tx, rx) = mpsc::unbounded_channel();

    // Create cancellation token
    let cancel_token = CancellationToken::new();

    // Create and start probe scheduler
    let scheduler = ProbeScheduler::new(
        vec![addr],
        config.interval,
        config.timeout,
        tx,
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Create and start metrics updater
    let updater = MetricsUpdater::new(
        store.clone(),
        rx,
        Duration::from_millis(100),
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        updater.run().await;
    });

    // Wait for several probe cycles
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Verify metrics were updated
    let store_read = store.read().await;
    assert_eq!(store_read.host_count(), 1, "Should have 1 host");

    let host = store_read.get_host(0).expect("Should have host at index 0");
    let sample_count = host.sample_count();

    // Should have at least a few samples (600ms / 100ms interval = ~6 probes)
    assert!(
        sample_count >= 3,
        "Should have at least 3 samples, got {}",
        sample_count
    );

    // Verify host has data
    assert!(host.has_data(), "Host should have data");

    // Check metrics are calculated
    let metrics = host.metrics();
    assert!(
        metrics.last_check.is_some(),
        "Should have last check timestamp"
    );

    // Clean shutdown
    cancel_token.cancel();
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// Test multi-host monitoring with different server states.
///
/// Verifies: System handles multiple hosts with different behaviors
#[tokio::test(flavor = "multi_thread")]
async fn test_multi_host_monitoring() {
    // Set up test TCP server (will succeed, auto-cleanup via RAII)
    let server = TestServer::start();
    let success_addr = server.addr().to_string();

    // Create configuration
    let config = ProbeConfig::new(
        Duration::from_millis(100),
        Duration::from_millis(50),
        Duration::from_secs(60),
        100,
    );

    // Create metrics store with two hosts: one succeeds, one fails
    let hosts = vec![
        (
            "Success Host".to_string(),
            success_addr.clone(),
            success_addr.clone(),
        ),
        (
            "Fail Host".to_string(),
            "127.0.0.1:1".to_string(),
            "127.0.0.1:1".to_string(),
        ), // Port 1 will fail
    ];
    let store = Arc::new(RwLock::new(MetricsStore::new(hosts, config.clone())));

    // Create channel
    let (tx, rx) = mpsc::unbounded_channel();
    let cancel_token = CancellationToken::new();

    // Start scheduler
    let scheduler = ProbeScheduler::new(
        vec![success_addr, "127.0.0.1:1".to_string()],
        config.interval,
        config.timeout,
        tx,
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Start updater
    let updater = MetricsUpdater::new(
        store.clone(),
        rx,
        Duration::from_millis(100),
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        updater.run().await;
    });

    // Wait for probes
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Verify both hosts tracked
    let store_read = store.read().await;
    assert_eq!(store_read.host_count(), 2, "Should have 2 hosts");

    let host0 = store_read.get_host(0).expect("Should have host 0");
    let host1 = store_read.get_host(1).expect("Should have host 1");

    // Both should have data
    assert!(host0.has_data(), "Host 0 should have data");
    assert!(host1.has_data(), "Host 1 should have data");

    // Host 0 (success) should have successful probes
    let metrics0 = host0.metrics();
    assert!(
        metrics0.success_rate > 0.0,
        "Success host should have some successful probes"
    );

    // Host 1 (fail) should have failures
    let metrics1 = host1.metrics();
    assert!(metrics1.failure_count > 0, "Fail host should have failures");

    // Clean shutdown
    cancel_token.cancel();
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// Test graceful shutdown.
///
/// Verifies: All components shut down cleanly when cancelled
#[tokio::test(flavor = "multi_thread")]
async fn test_graceful_shutdown() {
    // Set up test server (auto-cleanup via RAII)
    let server = TestServer::start();
    let addr = server.addr().to_string();

    let config = ProbeConfig::new(
        Duration::from_millis(50),
        Duration::from_millis(25),
        Duration::from_secs(60),
        100,
    );

    let hosts = vec![("Test".to_string(), addr.clone(), addr.clone())];
    let store = Arc::new(RwLock::new(MetricsStore::new(hosts, config.clone())));

    let (tx, rx) = mpsc::unbounded_channel();
    let cancel_token = CancellationToken::new();

    // Start components
    let scheduler = ProbeScheduler::new(
        vec![addr],
        config.interval,
        config.timeout,
        tx,
        cancel_token.clone(),
    );

    let scheduler_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    let updater = MetricsUpdater::new(
        store.clone(),
        rx,
        Duration::from_millis(50),
        cancel_token.clone(),
    );

    let updater_handle = tokio::spawn(async move {
        updater.run().await;
    });

    // Let them run briefly
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Signal cancellation
    cancel_token.cancel();

    // Wait for tasks to complete (with timeout)
    let scheduler_result = tokio::time::timeout(Duration::from_secs(2), scheduler_handle).await;
    let updater_result = tokio::time::timeout(Duration::from_secs(2), updater_handle).await;

    // Both should complete without timeout
    assert!(
        scheduler_result.is_ok(),
        "Scheduler should shut down cleanly"
    );
    assert!(updater_result.is_ok(), "Updater should shut down cleanly");

    // Verify no panics
    assert!(
        scheduler_result.unwrap().is_ok(),
        "Scheduler should not panic"
    );
    assert!(updater_result.unwrap().is_ok(), "Updater should not panic");
}

/// Test metrics store and updater interaction.
///
/// Verifies: Background updates process results and update store correctly
#[tokio::test(flavor = "multi_thread")]
async fn test_metrics_store_updater() {
    let config = ProbeConfig::new(
        Duration::from_millis(100),
        Duration::from_millis(50),
        Duration::from_secs(60),
        100,
    );

    let hosts = vec![(
        "Test Host".to_string(),
        "127.0.0.1:9999".to_string(),
        "127.0.0.1:9999".to_string(),
    )];
    let store = Arc::new(RwLock::new(MetricsStore::new(hosts, config.clone())));

    let (tx, rx) = mpsc::unbounded_channel();
    let cancel_token = CancellationToken::new();

    // Start metrics updater
    let updater = MetricsUpdater::new(
        store.clone(),
        rx,
        Duration::from_millis(50),
        cancel_token.clone(),
    );

    let updater_handle = tokio::spawn(async move {
        updater.run().await;
    });

    // Send several probe results manually
    use netglance::{ProbeError, ProbeResult};
    use std::time::Instant;

    let now = Instant::now();

    // Send some successes
    for i in 0..5 {
        tx.send((
            0,
            ProbeResult {
                timestamp: now + Duration::from_millis(i * 20),
                latency: Some(Duration::from_millis(10 + i)),
                error: None,
            },
        ))
        .unwrap();
    }

    // Send some failures
    for i in 0..3 {
        tx.send((
            0,
            ProbeResult {
                timestamp: now + Duration::from_millis(100 + i * 20),
                latency: None,
                error: Some(ProbeError::Timeout),
            },
        ))
        .unwrap();
    }

    // Wait for updater to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify store was updated
    let store_read = store.read().await;
    let host = store_read.get_host(0).expect("Should have host");

    assert_eq!(host.sample_count(), 8, "Should have 8 samples");

    let metrics = host.metrics();
    assert!(metrics.success_rate > 0.0, "Should have some successes");
    assert!(metrics.failure_count > 0, "Should have some failures");
    assert_eq!(metrics.failure_count, 3, "Should have exactly 3 failures");

    // Get snapshot to verify data is accessible
    let now = Instant::now();
    let snapshot = store_read.snapshot(now);
    assert_eq!(snapshot.hosts.len(), 1, "Should have 1 host in snapshot");

    // Clean shutdown
    drop(store_read);
    drop(tx); // Close channel to trigger shutdown
    let result = tokio::time::timeout(Duration::from_secs(2), updater_handle).await;
    assert!(
        result.is_ok(),
        "Updater should shut down when channel closes"
    );
}

/// Test real network integration with Google.
///
/// Verifies: Full stack works with real internet connectivity
#[tokio::test(flavor = "multi_thread")]
async fn test_real_network_integration() {
    let config = ProbeConfig::new(
        Duration::from_millis(200), // Slower for real network
        Duration::from_secs(2),     // Reasonable timeout for internet
        Duration::from_secs(60),
        100,
    );

    // Monitor Google HTTPS
    let hosts = vec![(
        "Google HTTPS".to_string(),
        "www.google.com:443".to_string(),
        "www.google.com:443".to_string(),
    )];
    let store = Arc::new(RwLock::new(MetricsStore::new(hosts, config.clone())));

    let (tx, rx) = mpsc::unbounded_channel();
    let cancel_token = CancellationToken::new();

    // Start scheduler
    let scheduler = ProbeScheduler::new(
        vec!["www.google.com:443".to_string()],
        config.interval,
        config.timeout,
        tx,
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Start updater
    let updater = MetricsUpdater::new(
        store.clone(),
        rx,
        Duration::from_millis(100),
        cancel_token.clone(),
    );

    tokio::spawn(async move {
        updater.run().await;
    });

    // Run for 10 probe cycles (10 * 200ms = 2 seconds)
    tokio::time::sleep(Duration::from_millis(2200)).await;

    // Verify results
    let store_read = store.read().await;
    let host = store_read.get_host(0).expect("Should have host");

    // Should have multiple samples
    assert!(
        host.sample_count() >= 8,
        "Should have at least 8 samples, got {}",
        host.sample_count()
    );

    let metrics = host.metrics();

    // Google should be very reliable
    assert!(
        metrics.success_rate >= 80.0,
        "Google should have high success rate, got {}%",
        metrics.success_rate
    );

    // Should have measured latency
    assert!(metrics.avg_latency.is_some(), "Should have average latency");

    // Latency should be reasonable (< 5 seconds)
    if let Some(avg) = metrics.avg_latency {
        assert!(
            avg < Duration::from_secs(5),
            "Average latency should be reasonable, got {:?}",
            avg
        );
    }

    // Verify overall metrics
    use std::time::Instant;
    let overall = store_read.get_overall_metrics(Instant::now());
    assert!(
        overall.overall_success_rate >= 80.0,
        "Overall success rate should be high"
    );

    // Clean shutdown
    cancel_token.cancel();
    tokio::time::sleep(Duration::from_millis(300)).await;
}
