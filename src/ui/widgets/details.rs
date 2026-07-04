//! Detail panel widget showing individual host details.

use crate::metrics::HostSnapshot;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

/// Render the detail panel for a selected host.
#[allow(dead_code)]
pub fn render_detail_panel(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    host: Option<&HostSnapshot>,
    _now: Instant,
) {
    if let Some(host) = host {
        // Split the detail area into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Status line
                Constraint::Min(3),    // Metrics
            ])
            .split(area);

        // Render status line
        render_status_line(frame, chunks[0], host);

        // Render metrics
        render_metrics(frame, chunks[1], host);
    } else {
        // No host selected
        let text = vec![Line::from(Span::styled(
            "No host selected",
            Style::default().fg(Color::DarkGray),
        ))];

        let block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Render the status line showing hostname and status.
fn render_status_line(frame: &mut Frame<'_>, area: ratatui::layout::Rect, host: &HostSnapshot) {
    let status_color = host.status.color();
    // Use the full indicator text for details view
    let status_text = host.status.indicator();

    let text = vec![Line::from(vec![
        Span::styled(
            host.hostname.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ("),
        Span::raw(host.address.clone()),
        Span::raw(") - "),
        Span::styled(
            status_text,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Render the metrics section.
fn render_metrics(frame: &mut Frame<'_>, area: ratatui::layout::Rect, host: &HostSnapshot) {
    let metrics = &host.metrics;

    let avg_latency = format_latency(metrics.avg_latency);
    let min_latency = format_latency(metrics.min_latency);
    let max_latency = format_latency(metrics.max_latency);
    let p95_latency = format_latency(metrics.p95_latency);

    let mut text = vec![
        Line::from(vec![
            Span::raw("Latency - Avg: "),
            Span::styled(avg_latency, Style::default().fg(Color::Cyan)),
            Span::raw(" | Min: "),
            Span::styled(min_latency, Style::default().fg(Color::Green)),
            Span::raw(" | Max: "),
            Span::styled(max_latency, Style::default().fg(Color::Red)),
            Span::raw(" | P95: "),
            Span::styled(p95_latency, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("Success Rate: "),
            Span::styled(
                format!("{:.2}%", metrics.success_rate),
                Style::default().fg(if metrics.success_rate >= 99.0 {
                    Color::Green
                } else if metrics.success_rate >= 95.0 {
                    Color::Yellow
                } else {
                    Color::Red
                }),
            ),
            Span::raw(" | Failures: "),
            Span::styled(
                metrics.failure_count.to_string(),
                Style::default().fg(if metrics.failure_count > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
            Span::raw(" | Consecutive: "),
            Span::styled(
                metrics.consecutive_failures.to_string(),
                Style::default().fg(if metrics.consecutive_failures > 0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("Last Success: "),
            Span::styled(
                format_instant(metrics.last_success),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" | Last Check: "),
            Span::styled(
                format_instant(metrics.last_check),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    // Add hint about logs if there are failures
    if metrics.failure_count > 0 {
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            "→ Check ~/.netglance/logs/netglance.log for error details",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    let block = Block::default().borders(Borders::NONE);
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Format a latency duration for display.
fn format_latency(latency: Option<Duration>) -> String {
    match latency {
        Some(d) => {
            let ms = d.as_secs_f64() * 1000.0;
            if ms < 1.0 {
                format!("{:.2} ms", ms)
            } else if ms < 100.0 {
                format!("{:.1} ms", ms)
            } else {
                format!("{:.0} ms", ms)
            }
        }
        None => "N/A".to_string(),
    }
}

/// Format an instant for display.
fn format_instant(instant: Option<Instant>) -> String {
    match instant {
        Some(t) => {
            let elapsed = Instant::now().duration_since(t);
            if elapsed.as_secs() < 60 {
                format!("{}s ago", elapsed.as_secs())
            } else if elapsed.as_secs() < 3600 {
                format!("{}m ago", elapsed.as_secs() / 60)
            } else {
                format!("{}h ago", elapsed.as_secs() / 3600)
            }
        }
        None => "Never".to_string(),
    }
}
