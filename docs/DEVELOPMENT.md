# Development Guide

This guide helps developers understand how to work with the netglance codebase, add features, and contribute changes.

## Quick Start

### Prerequisites

- **Rust:** 1.70+ (2021 edition)
- **Cargo:** Latest stable
- **Git:** For version control
- **Make:** For convenience commands (optional)

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/netglance.git
cd netglance

# Build the project
cargo build

# Run tests
make test

# Run the application
cargo run
```

### Development Workflow

```bash
# Fast iteration loop
make static          # Run all static checks
make test            # Run unit + integration tests
cargo run            # Test locally

# Before committing
make ci              # Full CI checks (fmt, clippy, test)
```

## Project Structure

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed component overview.

**Key directories:**
```
netglance/
├── src/              # Source code
│   ├── config/       # Configuration management
│   ├── metrics/      # Data structures and statistics
│   ├── probe/        # TCP probing infrastructure
│   └── ui/           # Terminal user interface
├── tests/            # Integration and network tests
├── docs/             # Documentation
└── Makefile          # Build and test shortcuts
```

## Common Development Tasks

### Adding a New Host Status

**File:** `src/metrics/status.rs`

1. Add variant to `HostStatus` enum:
```rust
pub enum HostStatus {
    Waiting,
    Healthy,
    Degraded,
    Down,
    Stale,
    NeverSucceeded,
    // Add new status here
    YourNewStatus,
}
```

2. Add classification logic in `HostStatus::classify()`:
```rust
pub fn classify(...) -> Self {
    // Your classification logic
    if some_condition {
        return HostStatus::YourNewStatus;
    }
    // ...
}
```

3. Add color and indicator:
```rust
pub fn color(&self) -> Color {
    match self {
        // ...
        HostStatus::YourNewStatus => Color::Magenta,
    }
}

pub fn short_indicator(&self) -> &'static str {
    match self {
        // ...
        HostStatus::YourNewStatus => "? NEW",
    }
}
```

4. Add test case:
```rust
#[test]
fn test_your_new_status() {
    let status = HostStatus::classify(/* params */);
    assert_eq!(status, HostStatus::YourNewStatus);
}
```

5. Run tests: `cargo test status`

### Adding a New Probe Type

**Current:** TCP connection only  
**Future:** HTTP health checks, ICMP ping, custom protocols

**Steps:**

1. Create new module: `src/probe/http.rs`
```rust
use crate::metrics::ProbeResult;
use std::time::Duration;

pub async fn probe_http(url: &str, timeout: Duration) -> ProbeResult {
    let start = Instant::now();
    
    match tokio::time::timeout(timeout, /* http request */).await {
        Ok(Ok(_)) => ProbeResult {
            timestamp: Instant::now(),
            latency: Some(start.elapsed()),
            error: None,
        },
        // Handle errors...
    }
}
```

2. Update `ProbeScheduler` in `src/probe/scheduler.rs`:
```rust
// Add protocol field to determine probe type
match host.protocol {
    Protocol::Tcp => probe_tcp(&addr, timeout).await,
    Protocol::Http => probe_http(&addr, timeout).await,
}
```

3. Update config schema in `src/config/schema.rs`:
```rust
pub struct HostConfig {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub protocol: Protocol,  // New field
    pub enabled: bool,
}

pub enum Protocol {
    Tcp,
    Http,
    // Future: Icmp, Custom
}
```

4. Add tests in `tests/`:
```rust
#[tokio::test]
async fn test_http_probe() {
    // Test HTTP probing logic
}
```

### Adding a New Metric

**Current metrics:** Latency (avg/min/max/p95), success rate, failure count  
**Future metrics:** Jitter, packet loss, uptime percentage, custom thresholds

**Steps:**

1. Update `HostMetrics` in `src/metrics/statistics.rs`:
```rust
pub struct HostMetrics {
    // Existing fields...
    pub avg_latency: Option<Duration>,
    
    // Add new metric
    pub jitter: Option<Duration>,
}
```

2. Add calculation in `calculate_metrics()`:
```rust
fn calculate_metrics(samples: &[ProbeResult]) -> HostMetrics {
    // Existing calculations...
    
    // Calculate jitter (variation in latency)
    let jitter = calculate_jitter(samples);
    
    HostMetrics {
        // ...
        jitter,
    }
}

fn calculate_jitter(samples: &[ProbeResult]) -> Option<Duration> {
    // Implementation
}
```

3. Update UI to display new metric in `src/ui/widgets/`:

**Table view** (`table.rs`):
```rust
Row::new(vec![
    Cell::from(host.hostname.clone()),
    // ...
    Cell::from(format_jitter(host.metrics.jitter)),
])
```

**Detail view** (`details.rs`):
```rust
Line::from(vec![
    Span::raw("Jitter: "),
    Span::styled(
        format_duration(host.metrics.jitter),
        Style::default().fg(Color::Yellow),
    ),
])
```

4. Add tests:
```rust
#[test]
fn test_jitter_calculation() {
    let samples = vec![/* test data */];
    let metrics = calculate_metrics(&samples);
    assert!(metrics.jitter.is_some());
}
```

### Adding a New UI View

**Current views:** Table (list all hosts), Detail (single host focus)  
**Future views:** Graph (latency over time), Comparison (side-by-side), Alerts

**Steps:**

1. Create widget: `src/ui/widgets/graph.rs`
```rust
use ratatui::{Frame, widgets::*};

pub fn render_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    host: &HostSnapshot,
) {
    // Use ratatui's Chart widget
    let datasets = vec![
        Dataset::default()
            .name("Latency")
            .data(&latency_data)
            .graph_type(GraphType::Line),
    ];
    
    let chart = Chart::new(datasets)
        .block(Block::default().title("Latency Graph"))
        .x_axis(Axis::default().title("Time"))
        .y_axis(Axis::default().title("ms"));
    
    frame.render_widget(chart, area);
}
```

2. Add to `src/ui/widgets/mod.rs`:
```rust
pub mod graph;
pub use graph::render_graph;
```

3. Add view mode to `src/ui/app.rs`:
```rust
pub enum ViewMode {
    Table,
    Details,
    Graph,  // New
}

impl App {
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('g') => {
                self.view_mode = ViewMode::Graph;
            }
            // ...
        }
    }
}
```

4. Update render logic in `src/ui/render.rs`:
```rust
match app.view_mode {
    ViewMode::Table => render_table(/* ... */),
    ViewMode::Details => render_details(/* ... */),
    ViewMode::Graph => render_graph(/* ... */),
}
```

5. Update help text in `src/ui/widgets/footer.rs`:
```rust
"g: Graph view | d: Details | t: Table | q: Quit"
```

## Testing

### Running Tests

```bash
# All tests
make test

# Unit tests only (fast, no network)
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Network tests only (requires internet)
cargo test --test network_tests

# Specific test
cargo test test_end_to_end_monitoring

# With output
cargo test -- --nocapture
```

### Test Organization

**Unit tests:** Located in `#[cfg(test)] mod tests` within source files
- Fast (<1s total)
- No network I/O
- Mock/fake implementations

**Integration tests:** Located in `tests/integration_tests.rs`
- Wire up multiple components
- Use `TestServer` helper for local TCP
- Verify end-to-end workflows

**Network tests:** Located in `tests/network_tests.rs`
- Require internet connection
- Test against real servers (www.google.com)
- Slow (~5s) but verify real-world behavior

### Writing New Tests

**Use the TestServer helper:**
```rust
use crate::helpers::TestServer;

#[tokio::test]
async fn test_my_feature() {
    // Automatic cleanup via RAII
    let server = TestServer::start();
    let addr = server.addr();
    
    // Your test logic...
    
    // Server automatically cleans up when dropped
}
```

**Test naming conventions:**
- `test_<functionality>` - Basic functionality test
- `test_<component>_<scenario>` - Specific scenario
- `test_<error_case>` - Error handling test

**Assertions:**
```rust
// Prefer descriptive messages
assert!(result.is_ok(), "Should succeed, got error: {:?}", result);

// Use assert_eq for specific values
assert_eq!(count, 5, "Should have 5 samples");

// Use matches! for enum variants
assert!(matches!(status, HostStatus::Healthy));
```

## Code Style

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt -- --check
```

**Style guide:** Follow [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)

### Linting

```bash
# Run clippy
cargo clippy

# Fix automatically when possible
cargo clippy --fix
```

**Key clippy rules:**
- Avoid `.unwrap()` in production code (use `?` or `.expect()` with context)
- Prefer `if let` over `match` for single pattern
- Use `#[must_use]` for functions that return important values

### Comments

**Default:** Write no comments. Code should be self-documenting.

**When to comment:**
- Non-obvious "why" (not "what")
- Workarounds for bugs in dependencies
- Performance optimizations that aren't obvious
- Complex algorithms with citations

**Example:**
```rust
// GOOD: Explains why
// Timeout must be less than interval to avoid overlapping probes
if timeout >= interval { ... }

// BAD: Explains what (code already says this)
// Set timeout to 5 seconds
let timeout = Duration::from_secs(5);
```

### Documentation

**Public API:** Always document with `///`
```rust
/// Calculate the 95th percentile of latency values.
///
/// Returns `None` if there are no successful probes.
pub fn percentile_95(samples: &[Duration]) -> Option<Duration> {
    // ...
}
```

**Modules:** Add module-level docs
```rust
//! TCP connection probing infrastructure.
//!
//! This module provides asynchronous TCP connection probing using Tokio.
```

## Debugging

### Logging

```bash
# Enable debug logs
RUST_LOG=debug cargo run

# Specific module
RUST_LOG=netglance::probe=debug cargo run

# Multiple modules
RUST_LOG=netglance::probe=debug,netglance::metrics=trace cargo run
```

**Log levels:**
- `error`: Something failed
- `warn`: Unusual but handled
- `info`: Normal operation (default)
- `debug`: Detailed for debugging
- `trace`: Very verbose

### Common Issues

**Issue:** Tests fail with "Address already in use"
- **Cause:** Previous test didn't clean up
- **Fix:** Use `TestServer` helper (automatic cleanup)

**Issue:** UI doesn't update
- **Cause:** Holding RwLock for too long
- **Fix:** Use snapshot pattern, release lock quickly

**Issue:** Probe timeouts
- **Cause:** Network latency or firewall
- **Fix:** Increase timeout in config, check firewall rules

## Release Process

### Version Bumping

1. Update version in `Cargo.toml`
2. Update `RELEASE_NOTES.md`
3. Commit: `git commit -am "Bump version to v0.3.0"`
4. Tag: `git tag -a v0.3.0 -m "Version 0.3.0"`

### Building Release

```bash
# Full CI checks
make ci

# Build optimized binary
cargo build --release

# Binary location
target/release/netglance
```

### GitHub Release

```bash
# Push tag
git push origin v0.3.0

# Create GitHub release (requires gh CLI)
make github-release VERSION=v0.3.0
```

## Performance Profiling

### CPU Profiling

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph

# Opens flamegraph.svg in browser
```

### Memory Profiling

```bash
# Install heaptrack
# Ubuntu: sudo apt install heaptrack

# Run with heaptrack
heaptrack target/release/netglance

# Analyze results
heaptrack_gui heaptrack.netglance.*.gz
```

### Benchmarking

Currently no benchmarks. To add:

1. Create `benches/` directory
2. Add criterion dependency
3. Write benchmark:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_metrics_calculation(c: &mut Criterion) {
    c.bench_function("calculate_metrics", |b| {
        b.iter(|| calculate_metrics(black_box(&samples)))
    });
}

criterion_group!(benches, benchmark_metrics_calculation);
criterion_main!(benches);
```

4. Run: `cargo bench`

## Contributing

### Before Submitting PR

- [ ] Run `make ci` (all checks pass)
- [ ] Add tests for new functionality
- [ ] Update documentation if needed
- [ ] Check that UI still works (manual testing)
- [ ] Ensure no clippy warnings

### PR Guidelines

- **Title:** Clear, descriptive (e.g., "Add HTTP probe support")
- **Description:** Explain what and why, not just what
- **Small PRs:** Prefer small, focused changes
- **Tests:** Include tests for new features
- **Breaking changes:** Clearly marked and justified

## Resources

- **Rust Book:** https://doc.rust-lang.org/book/
- **Tokio Tutorial:** https://tokio.rs/tokio/tutorial
- **Ratatui Examples:** https://github.com/ratatui-org/ratatui/tree/main/examples
- **Architecture:** See `docs/ARCHITECTURE.md`

---

**Questions?** Open an issue on GitHub or check existing documentation.

**Last Updated:** 2026-07-04  
**Version:** 0.2.0
