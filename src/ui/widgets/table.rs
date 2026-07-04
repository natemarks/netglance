//! Host table widget showing all monitored hosts.

use crate::metrics::HostSnapshot;
use crate::ui::app::SortMode;
use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::time::{Duration, Instant};

/// Render the host table with monitoring data.
#[allow(dead_code)]
pub fn render_host_table(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    mut hosts: Vec<HostSnapshot>,
    selected_idx: usize,
    sort_mode: SortMode,
    filter_failures: bool,
    now: Instant,
) {
    // Apply filtering
    if filter_failures {
        hosts.retain(|h| h.metrics.failure_count > 0);
    }

    // Apply sorting
    sort_hosts(&mut hosts, sort_mode);

    // Build table rows
    let header = Row::new(vec![
        Cell::from("Host"),
        Cell::from("Status"),
        Cell::from("Avg"),
        Cell::from("Min"),
        Cell::from("Max"),
        Cell::from("Success"),
        Cell::from("Fail"),
        Cell::from("Last"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = hosts
        .iter()
        .enumerate()
        .map(|(idx, host)| {
            let is_selected = idx == selected_idx;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(host.hostname.clone()),
                Cell::from(host.status.short_indicator()).style(Style::default().fg(host.status.color())),
                Cell::from(format_latency(host.metrics.avg_latency)),
                Cell::from(format_latency(host.metrics.min_latency)),
                Cell::from(format_latency(host.metrics.max_latency)),
                Cell::from(format!("{:.1}%", host.metrics.success_rate)),
                Cell::from(host.metrics.failure_count.to_string()).style(Style::default().fg(
                    if host.metrics.failure_count > 0 {
                        Color::Red
                    } else {
                        Color::Green
                    },
                )),
                Cell::from(format_last_check(host.metrics.last_check, now)),
            ])
            .style(style)
        })
        .collect();

    let title = if filter_failures {
        " Hosts (filtered) "
    } else {
        " Hosts "
    };

    let widths = [
        Constraint::Length(20), // Host
        Constraint::Length(8),  // Status
        Constraint::Length(10), // Avg
        Constraint::Length(10), // Min
        Constraint::Length(10), // Max
        Constraint::Length(9),  // Success
        Constraint::Length(6),  // Fail
        Constraint::Length(10), // Last
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

/// Sort hosts based on the sort mode.
fn sort_hosts(hosts: &mut [HostSnapshot], sort_mode: SortMode) {
    match sort_mode {
        SortMode::Name => {
            hosts.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        }
        SortMode::Latency => {
            hosts.sort_by(|a, b| {
                // Put hosts with no latency at the end
                match (a.metrics.avg_latency, b.metrics.avg_latency) {
                    (Some(a_lat), Some(b_lat)) => a_lat.cmp(&b_lat),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        SortMode::Failures => {
            hosts.sort_by_key(|b| std::cmp::Reverse(b.metrics.failure_count));
        }
    }
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

/// Format last check time relative to now.
fn format_last_check(last_check: Option<Instant>, now: Instant) -> String {
    match last_check {
        Some(t) => {
            let elapsed = now.duration_since(t);
            if elapsed.as_secs() < 60 {
                format!("{}s ago", elapsed.as_secs())
            } else {
                format!("{}m ago", elapsed.as_secs() / 60)
            }
        }
        None => "Never".to_string(),
    }
}
