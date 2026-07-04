//! Application state and event handling logic.

use crate::metrics::MetricsStore;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sort mode for the host table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// Sort by hostname (alphabetical)
    Name,
    /// Sort by average latency (ascending)
    Latency,
    /// Sort by failure count (descending)
    Failures,
}

impl SortMode {
    /// Get the next sort mode (cycles through Name -> Latency -> Failures -> Name)
    pub fn next(self) -> Self {
        match self {
            SortMode::Name => SortMode::Latency,
            SortMode::Latency => SortMode::Failures,
            SortMode::Failures => SortMode::Name,
        }
    }

    /// Get a display string for the current sort mode
    #[allow(dead_code)]
    pub fn display(&self) -> &'static str {
        match self {
            SortMode::Name => "Name",
            SortMode::Latency => "Latency",
            SortMode::Failures => "Failures",
        }
    }
}

/// Application state for the TUI.
pub struct App {
    /// Currently selected host index
    pub selected_host: usize,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Whether to show the detail panel
    pub show_details: bool,
    /// Whether to filter and show only failing hosts
    pub filter_failures: bool,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Shared metrics store
    pub metrics_store: Arc<RwLock<MetricsStore>>,
}

impl App {
    /// Create a new application state.
    pub fn new(metrics_store: Arc<RwLock<MetricsStore>>) -> Self {
        Self {
            selected_host: 0,
            should_quit: false,
            show_details: false,
            filter_failures: false,
            sort_mode: SortMode::Name,
            metrics_store,
        }
    }

    /// Handle a keyboard event.
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            // Quit with Ctrl+C
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            // Toggle details panel
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.show_details = !self.show_details;
            }
            // Toggle failure filter
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.filter_failures = !self.filter_failures;
            }
            // Cycle sort mode
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.sort_mode = self.sort_mode.next();
            }
            // Navigate up
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.selected_host > 0 {
                    self.selected_host -= 1;
                }
            }
            // Navigate down
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.selected_host += 1;
                // Will be clamped to actual host count during rendering
            }
            // Direct selection with number keys (1-5)
            KeyCode::Char('1') => self.selected_host = 0,
            KeyCode::Char('2') => self.selected_host = 1,
            KeyCode::Char('3') => self.selected_host = 2,
            KeyCode::Char('4') => self.selected_host = 3,
            KeyCode::Char('5') => self.selected_host = 4,
            _ => {}
        }
    }

    /// Get the total number of hosts being monitored.
    pub async fn host_count(&self) -> usize {
        self.metrics_store.read().await.host_count()
    }

    /// Clamp the selected host index to valid range.
    pub async fn clamp_selection(&mut self) {
        let count = self.host_count().await;
        if count > 0 && self.selected_host >= count {
            self.selected_host = count - 1;
        }
    }
}
