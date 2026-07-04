//! Rendering logic for the TUI application.

use super::{app::App, layout::AppLayout, widgets};
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

/// Render the entire application UI.
pub async fn render(frame: &mut Frame<'_>, app: &mut App) {
    // Clamp selection to valid range
    app.clamp_selection().await;

    // Get current time for relative timestamps
    let now = Instant::now();

    // Get metrics snapshot from store
    let store = app.metrics_store.read().await;
    let snapshot = store.snapshot(now);
    let config = store.config().clone();
    drop(store); // Release lock

    // Calculate layout based on host count
    let layout = AppLayout::new(frame, snapshot.hosts.len());

    // Render summary bar
    widgets::render_summary_bar(
        frame,
        layout.summary,
        &snapshot.overall,
        config.window_duration,
    );

    // Render host panels (one per host, up to 5)
    for (i, host) in snapshot.hosts.iter().enumerate() {
        if let Some(area) = layout.host_panels.get(i) {
            widgets::render_host_panel(frame, *area, host);
        }
    }

    // Render help bar
    render_help(frame, &layout);
}

/// Render the help bar.
fn render_help(frame: &mut Frame<'_>, layout: &AppLayout) {
    // Get version with git hash from build-time environment variables
    let version = env!("BUILD_VERSION");
    let git_hash = env!("GIT_HASH");
    let version_str = format!("v{}-{}", version, git_hash);

    let help_text = format!("[q]uit [s]ort [f]filter [r]efresh | {}", version_str);

    let paragraph = Paragraph::new(Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )]))
    .alignment(Alignment::Center);

    frame.render_widget(paragraph, layout.help);
}
