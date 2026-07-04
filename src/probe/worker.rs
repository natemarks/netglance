//! TCP connection probe worker.
//!
//! Performs individual TCP connection attempts and measures latency.

use std::io;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, warn};

use crate::metrics::{ProbeError, ProbeResult};

/// Attempt a TCP connection to the specified address and measure latency.
///
/// # Arguments
/// * `address` - The host:port to connect to (e.g., "192.168.1.1:443")
/// * `timeout_duration` - Maximum time to wait for connection
///
/// # Returns
/// A `ProbeResult` containing either the latency or an error.
pub async fn probe_tcp(address: &str, timeout_duration: Duration) -> ProbeResult {
    let start = Instant::now();

    debug!("Probing {}", address);

    // Attempt connection with timeout
    let result = timeout(timeout_duration, TcpStream::connect(address)).await;

    match result {
        Ok(Ok(_stream)) => {
            // Connection successful
            let latency = start.elapsed();
            debug!("Probe succeeded to {} in {:?}", address, latency);
            ProbeResult::success(start, latency)
        }
        Ok(Err(e)) => {
            // Connection failed (but didn't timeout)
            let error = map_io_error(e);
            warn!("Probe failed to {}: {}", address, error);
            ProbeResult::failure(start, error)
        }
        Err(_) => {
            // Timeout occurred
            warn!("Probe timeout to {} after {:?}", address, timeout_duration);
            ProbeResult::failure(start, ProbeError::Timeout)
        }
    }
}

/// Map std::io::Error to ProbeError.
fn map_io_error(error: io::Error) -> ProbeError {
    use io::ErrorKind;

    match error.kind() {
        ErrorKind::ConnectionRefused => ProbeError::ConnectionRefused,
        ErrorKind::TimedOut => ProbeError::Timeout,
        ErrorKind::NotFound => ProbeError::DnsFailure,
        ErrorKind::InvalidInput if error.to_string().contains("invalid socket address") => {
            ProbeError::DnsFailure
        }
        _ => {
            // Check if it's a DNS-related error by examining the error message
            let error_msg = error.to_string();
            if error_msg.contains("failed to lookup address")
                || error_msg.contains("Name or service not known")
                || error_msg.contains("nodename nor servname provided")
            {
                ProbeError::DnsFailure
            } else {
                ProbeError::IoError(error_msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[tokio::test]
    async fn probe_successful_connection() {
        // Start a local TCP server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a task to accept the connection
        tokio::spawn(async move {
            let _ = listener.accept();
        });

        // Probe the server
        let result = probe_tcp(&addr.to_string(), Duration::from_secs(1)).await;

        assert!(result.is_success());
        assert!(result.latency.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn probe_connection_refused() {
        // Use a port that's not listening
        let result = probe_tcp("127.0.0.1:1", Duration::from_secs(1)).await;

        assert!(result.is_failure());
        assert!(result.latency.is_none());
        assert_eq!(result.error, Some(ProbeError::ConnectionRefused));
    }

    #[tokio::test]
    async fn probe_timeout() {
        // Use an IP that will timeout (reserved IP that doesn't route)
        let result = probe_tcp("192.0.2.1:80", Duration::from_millis(100)).await;

        assert!(result.is_failure());
        assert!(result.latency.is_none());
        assert_eq!(result.error, Some(ProbeError::Timeout));
    }

    #[tokio::test]
    async fn probe_dns_failure() {
        // Use an invalid hostname
        let result = probe_tcp(
            "invalid.host.that.does.not.exist.local:80",
            Duration::from_secs(1),
        )
        .await;

        assert!(result.is_failure());
        assert!(result.latency.is_none());
        // Should be DNS failure, but can also timeout if DNS resolver is slow
        match result.error {
            Some(ProbeError::DnsFailure | ProbeError::Timeout) => (),
            other => panic!("Expected DnsFailure or Timeout, got {:?}", other),
        }
    }

    #[test]
    fn map_io_error_connection_refused() {
        let error = io::Error::from(io::ErrorKind::ConnectionRefused);
        assert_eq!(map_io_error(error), ProbeError::ConnectionRefused);
    }

    #[test]
    fn map_io_error_timeout() {
        let error = io::Error::from(io::ErrorKind::TimedOut);
        assert_eq!(map_io_error(error), ProbeError::Timeout);
    }

    #[test]
    fn map_io_error_not_found() {
        let error = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(map_io_error(error), ProbeError::DnsFailure);
    }
}
