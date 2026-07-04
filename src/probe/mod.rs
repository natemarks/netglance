//! TCP connection probing infrastructure.
//!
//! This module provides asynchronous TCP connection probing using Tokio,
//! including worker functions and scheduling logic.

pub mod dns;
pub mod scheduler;
pub mod worker;

// Public re-exports for library API
pub use dns::resolve_address;
pub use scheduler::ProbeScheduler;
pub use worker::probe_tcp;
