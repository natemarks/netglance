//! Test helpers for integration testing.
//!
//! Provides utilities for setting up test TCP servers with automatic cleanup.

use std::net::TcpListener;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Test TCP server that accepts connections.
///
/// Automatically cleans up when dropped (RAII pattern).
///
/// # Example
///
/// ```no_run
/// # use tests::helpers::TestServer;
/// # #[tokio::test]
/// # async fn example() {
/// let server = TestServer::start();
/// let addr = server.addr();
///
/// // Use addr in tests...
/// // Server automatically shuts down when dropped
/// # }
/// ```
pub struct TestServer {
    addr: String,
    _cancel: CancellationToken,
    _handle: JoinHandle<()>,
}

impl TestServer {
    /// Start a new test TCP server on a random port.
    ///
    /// The server accepts all connections and immediately drops them.
    /// Port is automatically assigned by binding to 127.0.0.1:0.
    ///
    /// # Panics
    ///
    /// Panics if unable to bind to a port or start the server task.
    /// This is appropriate for test code.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test server");
        let addr = listener
            .local_addr()
            .expect("Failed to get local address")
            .to_string();
        listener
            .set_nonblocking(true)
            .expect("Failed to set nonblocking");

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move {
            let listener =
                TokioTcpListener::from_std(listener).expect("Failed to convert to tokio listener");

            loop {
                tokio::select! {
                    Ok((_socket, _addr)) = listener.accept() => {
                        // Accept and immediately drop - simulates successful connection
                    }
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                }
            }
        });

        Self {
            addr,
            _cancel: cancel,
            _handle: handle,
        }
    }

    /// Get the address of the test server.
    ///
    /// Returns address in format "127.0.0.1:PORT".
    pub fn addr(&self) -> &str {
        &self.addr
    }
}

// Automatic cleanup via Drop trait (RAII)
impl Drop for TestServer {
    fn drop(&mut self) {
        self._cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_server_starts_and_accepts() {
        let server = TestServer::start();

        // Should be able to connect
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            tokio::net::TcpStream::connect(server.addr()),
        )
        .await;

        assert!(result.is_ok(), "Should connect to test server");
        assert!(result.unwrap().is_ok(), "Connection should succeed");
    }

    #[tokio::test]
    async fn test_server_cleanup_on_drop() {
        let addr = {
            let server = TestServer::start();
            server.addr().to_string()
        }; // server dropped here

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connection should fail after server is dropped
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            tokio::net::TcpStream::connect(&addr),
        )
        .await;

        assert!(
            result.is_err() || result.unwrap().is_err(),
            "Connection should fail after server cleanup"
        );
    }

    #[tokio::test]
    async fn test_multiple_servers() {
        let server1 = TestServer::start();
        let server2 = TestServer::start();

        // Both should have different addresses
        assert_ne!(
            server1.addr(),
            server2.addr(),
            "Servers should use different ports"
        );

        // Both should accept connections
        let result1 = tokio::net::TcpStream::connect(server1.addr()).await;
        let result2 = tokio::net::TcpStream::connect(server2.addr()).await;

        assert!(result1.is_ok(), "Server 1 should accept connections");
        assert!(result2.is_ok(), "Server 2 should accept connections");
    }

    #[tokio::test]
    async fn test_server_handles_multiple_connections() {
        let server = TestServer::start();

        // Connect multiple times in sequence
        for _ in 0..5 {
            let result = tokio::net::TcpStream::connect(server.addr()).await;
            assert!(result.is_ok(), "Server should handle multiple connections");
        }
    }
}
