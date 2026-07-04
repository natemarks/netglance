//! Summary bar widget showing overall metrics.

use crate::metrics::OverallMetrics;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Duration;

/// Render the summary bar with overall metrics.
pub fn render_summary_bar(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    overall: &OverallMetrics,
    window_duration: Duration,
) {
    let window_mins = window_duration.as_secs() / 60;

    // Format latency values
    let avg_latency_str = format_latency(overall.avg_latency);
    let p95_latency_str = format_latency(overall.p95_latency);

    // Color code success rate
    let success_color = if overall.overall_success_rate >= 99.0 {
        Color::Green
    } else if overall.overall_success_rate >= 95.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let text = vec![
        Line::from(vec![
            Span::styled(
                "netglance",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - TCP Connection Monitor | Window: "),
            Span::styled(
                format!("{}m", window_mins),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Success: "),
            Span::styled(
                format!("{:.1}%", overall.overall_success_rate),
                Style::default().fg(success_color),
            ),
            Span::raw(" | Avg: "),
            Span::styled(avg_latency_str, Style::default().fg(Color::Cyan)),
            Span::raw(" | P95: "),
            Span::styled(p95_latency_str, Style::default().fg(Color::Cyan)),
            Span::raw(" | Failures: "),
            Span::styled(
                format!("{}/min", overall.failure_rate_per_minute as u32),
                Style::default().fg(if overall.failure_rate_per_minute > 0.0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);

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
