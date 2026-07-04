//! netglance - TCP Connection Monitor TUI with Latency Tracking
//!
//! A Rust-based terminal user interface (TUI) application for monitoring TCP connections
//! to multiple hosts, tracking latency, and displaying health metrics in real-time.
//!
//! # Architecture
//!
//! The application is structured into several key modules:
//!
//! - **config**: Configuration structures for probe behavior and timing
//! - **metrics**: Data collection, statistics, and health status determination
//! - **probe**: Asynchronous TCP connection probing infrastructure
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use netglance::{ProbeConfig, MetricsStore, ProbeScheduler, probe_tcp};
//! use std::sync::Arc;
//! use std::time::Duration;
//! use tokio::sync::{mpsc, RwLock};
//! use tokio_util::sync::CancellationToken;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Configure probe behavior
//!     let config = ProbeConfig::new(
//!         Duration::from_secs(1),    // probe interval
//!         Duration::from_secs(5),    // timeout per probe
//!         Duration::from_secs(600),  // 10 minute rolling window
//!         600,                       // max samples per host
//!     );
//!
//!     // Set up monitoring for hosts (name, address, resolved_address)
//!     let hosts = vec![
//!         ("Google DNS".to_string(), "8.8.8.8:53".to_string(), "8.8.8.8:53".to_string()),
//!         ("Cloudflare DNS".to_string(), "1.1.1.1:53".to_string(), "1.1.1.1:53".to_string()),
//!     ];
//!
//!     let store = Arc::new(RwLock::new(MetricsStore::new(hosts.clone(), config.clone())));
//!
//!     // Create channel for probe results
//!     let (tx, rx) = mpsc::unbounded_channel();
//!     let cancel_token = CancellationToken::new();
//!
//!     // Start probe scheduler (use resolved addresses)
//!     let addresses: Vec<String> = hosts.iter().map(|(_, _, resolved)| resolved.clone()).collect();
//!     let scheduler = ProbeScheduler::new(
//!         addresses,
//!         config.interval,
//!         config.timeout,
//!         tx,
//!         cancel_token.clone(),
//!     );
//!
//!     // Run monitoring (in real app, also start MetricsUpdater and UI)
//!     tokio::spawn(async move {
//!         scheduler.run().await;
//!     });
//!
//!     // ... rest of application
//! }
//! ```

// Re-export public configuration
pub use config::ProbeConfig;

// Re-export metrics types
pub use metrics::{
    HostMetrics, HostSnapshot, HostState, HostStatus, MetricsSnapshot, MetricsStore,
    MetricsUpdater, OverallMetrics, ProbeError, ProbeResult,
};

// Re-export probe infrastructure
pub use probe::{probe_tcp, ProbeScheduler};

// Module declarations
pub mod config;
pub mod constants;
pub mod logging;
pub mod metrics;
pub mod probe;
