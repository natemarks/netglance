//! Network integration tests against real services.
//!
//! These tests assume internet connectivity is available.
//! They test against www.google.com:443 which is assumed to be stable and reliable.
//!
//! Note: These tests verify real-world network behavior that cannot be fully
//! replicated with local mock servers (DNS resolution, actual network latency,
//! timeout behavior with real TCP stacks, etc.).

use netglance::metrics::ProbeError;
use netglance::probe::probe_tcp;
use std::time::Duration;

/// Test successful HTTPS connection to Google.
///
/// Verifies that we can successfully connect to a reliable internet endpoint
/// and measure latency accurately.
#[tokio::test]
async fn test_real_https_connection() {
    let result = probe_tcp("www.google.com:443", Duration::from_secs(5)).await;

    assert!(
        result.latency.is_some(),
        "Should successfully connect to www.google.com:443, got error: {:?}",
        result.error
    );

    // Latency should be reasonable (under 2 seconds for any internet connection)
    let latency = result.latency.unwrap();
    assert!(
        latency < Duration::from_secs(2),
        "Latency should be under 2s, got {:?}",
        latency
    );
}

/// Test timeout behavior with a very short timeout.
///
/// Uses a very short timeout to test timeout handling. On fast networks this
/// may still succeed, which is acceptable - we're testing the timeout mechanism
/// works when needed.
#[tokio::test]
async fn test_real_connection_timeout() {
    let result = probe_tcp("www.google.com:443", Duration::from_millis(1)).await;

    // This will likely timeout due to very short duration.
    // But if it succeeds (very fast network), that's also acceptable.
    if result.latency.is_none() {
        assert!(
            matches!(result.error, Some(ProbeError::Timeout)),
            "Short timeout should produce Timeout error, got: {:?}",
            result.error
        );
    }
    // If it succeeded, the network is just that fast - also valid
}

/// Test DNS failure with an invalid hostname.
///
/// Verifies that DNS resolution failures are correctly detected and reported.
#[tokio::test]
async fn test_invalid_hostname() {
    let result = probe_tcp(
        "this-host-definitely-does-not-exist-12345.invalid:443",
        Duration::from_secs(5),
    )
    .await;

    assert!(result.latency.is_none(), "Should fail for invalid hostname");
    assert!(
        matches!(result.error, Some(ProbeError::DnsFailure)),
        "Should return DNS failure, got: {:?}",
        result.error
    );
}

/// Test connection to an uncommon port that's likely closed.
///
/// Port 81 is typically not open on Google's servers, so this should fail.
/// It may timeout or be refused depending on firewall configuration.
#[tokio::test]
async fn test_connection_refused_real() {
    let result = probe_tcp("www.google.com:81", Duration::from_secs(5)).await;

    // This will either timeout or be refused depending on firewall.
    // Either way, it should fail.
    assert!(
        result.latency.is_none(),
        "Should fail for closed port, got success with latency: {:?}",
        result.latency
    );
}

/// Test multiple sequential probes to verify reliability.
///
/// Verifies that consecutive probes to the same endpoint all succeed,
/// demonstrating stable connection behavior.
#[tokio::test]
async fn test_multiple_sequential_probes() {
    for i in 0..5 {
        let result = probe_tcp("www.google.com:443", Duration::from_secs(5)).await;
        assert!(
            result.latency.is_some(),
            "Sequential probe {} should succeed, got error: {:?}",
            i + 1,
            result.error
        );
    }
}

/// Test concurrent probes to verify parallel execution.
///
/// Spawns 10 concurrent probes and verifies that most succeed, demonstrating
/// that the async probe implementation correctly handles concurrent operations.
///
/// We allow some tolerance (8/10) for network variability.
#[tokio::test]
async fn test_concurrent_real_probes() {
    let mut tasks = vec![];

    for _ in 0..10 {
        tasks.push(tokio::spawn(async {
            probe_tcp("www.google.com:443", Duration::from_secs(5)).await
        }));
    }

    let results: Vec<_> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.expect("Task should not panic"))
        .collect();

    let successful = results.iter().filter(|r| r.latency.is_some()).count();

    // At least 80% should succeed (allow for some network variability)
    assert!(
        successful >= 8,
        "At least 8/10 concurrent probes should succeed, got {}. Failures: {:?}",
        successful,
        results
            .iter()
            .filter(|r| r.latency.is_none())
            .map(|r| &r.error)
            .collect::<Vec<_>>()
    );
}
