//! TCP connection probing infrastructure.
//!
//! This module provides asynchronous TCP connection probing using Tokio,
//! including worker functions and scheduling logic.

pub mod dns;
pub mod scheduler;
pub mod worker;

// Public re-exports for library API
pub use scheduler::ProbeScheduler;

// Re-exported for library users and tests, not used by binary
#[allow(unused_imports)]
pub use worker::probe_tcp;
