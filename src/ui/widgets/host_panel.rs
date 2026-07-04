//! Compact host panel widget for multi-endpoint dashboard.

use crate::metrics::{HostSnapshot, ProbeResult};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Duration;

/// Render a compact host panel showing status, metrics, and sparklines.
pub fn render_host_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, host: &HostSnapshot) {
    let never_succeeded = host.metrics.success_rate == 0.0 && host.metrics.total_probes > 5;

    // Determine border color and style based on status
    let (border_color, bg_color) = if never_succeeded {
        (Color::Red, Some(Color::Red))
    } else {
        (host.status.color(), None)
    };

    // Create title with status indicator
    let status_text = host.status.indicator();

    // Show resolved IP if different from original address
    let address_display = if host.address != host.resolved_address {
        // DNS name was resolved
        format!("{} → {}", host.address, host.resolved_address)
    } else {
        // Already an IP
        host.resolved_address.clone()
    };

    let title = format!(
        " {} ({}) ─ {} ",
        host.hostname, address_display, status_text
    );

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )
        .style(if let Some(bg) = bg_color {
            Style::default().bg(bg).fg(Color::White)
        } else {
            Style::default()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into 4 lines: metrics, latency sparkline, status timeline, warning/info
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Metrics summary
            Constraint::Length(1), // Latency sparkline
            Constraint::Length(1), // Status timeline
            Constraint::Length(1), // Warning/info line
        ])
        .split(inner);

    if never_succeeded {
        render_never_succeeded_content(frame, &chunks, host, bg_color.is_some());
    } else {
        render_normal_content(frame, &chunks, host);
    }
}

/// Render content for hosts that have never succeeded.
fn render_never_succeeded_content(
    frame: &mut Frame<'_>,
    chunks: &[ratatui::layout::Rect],
    host: &HostSnapshot,
    use_white_text: bool,
) {
    let text_style = if use_white_text {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let metrics_line = Line::from(vec![
        Span::styled("⚠ NEVER SUCCEEDED ", text_style),
        Span::styled(
            format!("({} failed attempts)", host.metrics.failure_count),
            if use_white_text {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Red)
            },
        ),
    ]);

    let latency_line = Line::from(Span::styled(
        "Latency: (no successful probes)",
        if use_white_text {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        },
    ));

    let status_line = Line::from(Span::styled(
        format!("Status:  {}", "✗".repeat(60.min(host.metrics.total_probes))),
        if use_white_text {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Red)
        },
    ));

    let warning_line = Line::from(Span::styled(
        "Check: DNS resolution? Port open? Firewall rules? Network reachable?",
        if use_white_text {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC)
        },
    ));

    frame.render_widget(Paragraph::new(metrics_line), chunks[0]);
    frame.render_widget(Paragraph::new(latency_line), chunks[1]);
    frame.render_widget(Paragraph::new(status_line), chunks[2]);
    frame.render_widget(Paragraph::new(warning_line), chunks[3]);
}

/// Render normal host content with sparklines.
fn render_normal_content(
    frame: &mut Frame<'_>,
    chunks: &[ratatui::layout::Rect],
    host: &HostSnapshot,
) {
    // Metrics summary line
    let metrics_line = create_metrics_line(host);
    frame.render_widget(Paragraph::new(metrics_line), chunks[0]);

    // Latency sparkline
    let latency_line = create_latency_sparkline(host);
    frame.render_widget(Paragraph::new(latency_line), chunks[1]);

    // Status timeline
    let status_line = create_status_timeline(host);
    frame.render_widget(Paragraph::new(status_line), chunks[2]);

    // Info/warning line
    let info_line = create_info_line(host);
    frame.render_widget(Paragraph::new(info_line), chunks[3]);
}

/// Create the metrics summary line.
fn create_metrics_line(host: &HostSnapshot) -> Line<'static> {
    let avg = format_latency(host.metrics.avg_latency);
    let min = format_latency(host.metrics.min_latency);
    let max = format_latency(host.metrics.max_latency);

    Line::from(vec![
        Span::raw("Avg: "),
        Span::styled(avg, Style::default().fg(Color::Cyan)),
        Span::raw(" | Min: "),
        Span::styled(min, Style::default().fg(Color::Green)),
        Span::raw(" | Max: "),
        Span::styled(max, Style::default().fg(Color::Red)),
        Span::raw(" | Success: "),
        Span::styled(
            format!("{:.1}%", host.metrics.success_rate),
            Style::default().fg(if host.metrics.success_rate >= 99.0 {
                Color::Green
            } else if host.metrics.success_rate >= 95.0 {
                Color::Yellow
            } else {
                Color::Red
            }),
        ),
        Span::raw(" | Failures: "),
        Span::styled(
            host.metrics.failure_count.to_string(),
            Style::default().fg(if host.metrics.failure_count > 0 {
                Color::Red
            } else {
                Color::Green
            }),
        ),
    ])
}

/// Create latency sparkline using block characters.
fn create_latency_sparkline(host: &HostSnapshot) -> Line<'static> {
    if host.recent_samples.is_empty() {
        return Line::from(Span::styled(
            "Latency: (no data)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Bucket samples into 60 time slots
    let sparkline = create_sparkline_bars(&host.recent_samples, 60);

    // Find max latency for scale display
    let max_latency = host
        .recent_samples
        .iter()
        .filter_map(|s| s.latency)
        .max()
        .unwrap_or(Duration::from_millis(100));

    Line::from(vec![
        Span::raw("Latency: "),
        Span::styled(sparkline, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!(" (0-{})", format_latency_short(max_latency)),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

/// Create status timeline showing success/failure markers.
fn create_status_timeline(host: &HostSnapshot) -> Line<'static> {
    if host.recent_samples.is_empty() {
        return Line::from(Span::styled(
            "Status:  (no data)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Create timeline with success/failure markers
    let mut timeline = String::with_capacity(60);
    let bucket_size = if host.recent_samples.len() > 60 {
        host.recent_samples.len() / 60
    } else {
        1
    };

    for chunk in host.recent_samples.chunks(bucket_size) {
        // Stop if we've reached target width (in characters, not bytes)
        if timeline.chars().count() >= 60 {
            break;
        }

        if chunk.iter().any(|s| s.is_success()) {
            timeline.push('●');
        } else {
            timeline.push('✗');
        }
    }

    // Pad if needed (use character count, not byte length)
    while timeline.chars().count() < 60 {
        timeline.push('·');
    }

    // Truncate to exact character width if too long
    let char_count = timeline.chars().count();
    if char_count > 60 {
        timeline = timeline.chars().take(60).collect();
    }

    // Color based on recent failures
    let color = if host.metrics.consecutive_failures > 0 {
        Color::Red
    } else if host.metrics.failure_count > 0 {
        Color::Yellow
    } else {
        Color::Green
    };

    Line::from(vec![
        Span::raw("Status:  "),
        Span::styled(timeline, Style::default().fg(color)),
        Span::raw("→"),
    ])
}

/// Create info/warning line.
fn create_info_line(host: &HostSnapshot) -> Line<'static> {
    if host.metrics.consecutive_failures >= 3 {
        Line::from(Span::styled(
            format!(
                "⚠ {} consecutive failures - check logs for details",
                host.metrics.consecutive_failures
            ),
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::ITALIC),
        ))
    } else if host.metrics.failure_count > 0 {
        Line::from(Span::styled(
            "→ Check ~/.netglance/logs/netglance.log for error details",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from(Span::styled(
            format!("Last checked: {}", format_instant(host.metrics.last_check)),
            Style::default().fg(Color::DarkGray),
        ))
    }
}

/// Create sparkline bars using Unicode block characters.
fn create_sparkline_bars(samples: &[ProbeResult], target_width: usize) -> String {
    if samples.is_empty() {
        return String::new();
    }

    let bars = "▁▂▃▄▅▆▇█";

    // Find max latency for scaling
    let max_latency = samples
        .iter()
        .filter_map(|s| s.latency)
        .max()
        .unwrap_or(Duration::from_millis(1));

    let max_ms = max_latency.as_secs_f64() * 1000.0;

    // Bucket samples if we have more than target width
    let bucket_size = if samples.len() > target_width {
        samples.len() / target_width
    } else {
        1
    };

    let mut result = String::with_capacity(target_width);

    for chunk in samples.chunks(bucket_size) {
        // Stop if we've reached target width (in characters, not bytes)
        if result.chars().count() >= target_width {
            break;
        }

        // Get average latency for this bucket
        let latencies: Vec<_> = chunk.iter().filter_map(|s| s.latency).collect();

        if latencies.is_empty() {
            result.push(' '); // Failed probe = gap
        } else {
            let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            let ms = avg_latency.as_secs_f64() * 1000.0;
            let ratio = ms / max_ms;
            let index = ((ratio * 7.0).min(7.0) as usize).min(7);
            result.push(bars.chars().nth(index).unwrap());
        }
    }

    // Pad if needed (use character count, not byte length)
    while result.chars().count() < target_width {
        result.push('▁');
    }

    // Truncate to exact character width if too long
    let char_count = result.chars().count();
    if char_count > target_width {
        result = result.chars().take(target_width).collect();
    }

    result
}

/// Format a latency duration for display.
fn format_latency(latency: Option<Duration>) -> String {
    match latency {
        Some(d) => {
            let ms = d.as_secs_f64() * 1000.0;
            if ms < 1.0 {
                format!("{:.2}ms", ms)
            } else if ms < 100.0 {
                format!("{:.1}ms", ms)
            } else {
                format!("{:.0}ms", ms)
            }
        }
        None => "N/A".to_string(),
    }
}

/// Format latency for short display (scale indicator).
fn format_latency_short(latency: Duration) -> String {
    let ms = latency.as_secs_f64() * 1000.0;
    if ms < 1000.0 {
        format!("{:.0}ms", ms)
    } else {
        format!("{:.1}s", ms / 1000.0)
    }
}

/// Format an instant for display.
fn format_instant(instant: Option<std::time::Instant>) -> String {
    match instant {
        Some(t) => {
            let elapsed = std::time::Instant::now().duration_since(t);
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
