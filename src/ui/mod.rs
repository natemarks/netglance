//! Terminal User Interface components.
//!
//! This module provides the TUI infrastructure using Ratatui and Crossterm,
//! including terminal setup, event handling, application state, and rendering.

pub mod app;
pub mod events;
pub mod layout;
pub mod render;
pub mod terminal;
pub mod widgets;

pub use app::App;
pub use events::{Event, EventHandler};
pub use terminal::{restore_terminal, setup_terminal};
