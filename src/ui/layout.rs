//! Layout definition and rendering structure.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Layout sections for the TUI.
#[derive(Debug)]
pub struct AppLayout {
    /// Summary bar area (top, fixed 3 lines)
    pub summary: Rect,
    /// Host panels (up to 5, 6 lines each)
    pub host_panels: Vec<Rect>,
    /// Help bar area (bottom, fixed 1 line)
    pub help: Rect,
}

impl AppLayout {
    /// Calculate layout for multi-panel view.
    pub fn new(frame: &Frame, host_count: usize) -> Self {
        let area = frame.area();

        // Calculate constraints: summary (3) + N*hosts (6 each) + help (1)
        let mut constraints = vec![Constraint::Length(3)]; // Summary

        // Add one panel per host (up to 5, 6 lines each)
        let panel_count = host_count.min(5);
        for _ in 0..panel_count {
            constraints.push(Constraint::Length(6)); // Each host panel: 6 lines
        }

        constraints.push(Constraint::Length(1)); // Help bar

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        Self {
            summary: chunks[0],
            host_panels: chunks[1..=panel_count].to_vec(),
            help: chunks[panel_count + 1],
        }
    }
}
