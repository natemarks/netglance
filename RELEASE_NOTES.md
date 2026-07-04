# netglance v0.1.0 - Release Notes

**Release Date**: 2026-07-03

## Overview

netglance is a high-performance TCP connection monitoring tool with a beautiful terminal user interface (TUI). This initial release provides comprehensive monitoring capabilities for up to 5 hosts simultaneously, with real-time latency tracking, failure detection, and rolling statistics.

## Features

### Core Functionality

✅ **Real-time TCP Monitoring**
- Monitor up to 5 hosts simultaneously
- Configurable probe intervals (default: 1 second)
- Configurable timeout per probe (default: 5 seconds)
- Rolling 10-minute statistics window (configurable)

✅ **Comprehensive Metrics**
- Average, minimum, maximum latency
- 95th percentile (P95) latency
- Success rate percentage
- Failure count and consecutive failures tracking
- Overall system health metrics

✅ **Beautiful Terminal UI**
- Clean, organized layout with 4 sections
- Color-coded status indicators (OK/WARN/DOWN/STALE)
- Real-time updates at 10 FPS
- Keyboard-driven navigation
- Toggleable detail panel
- Sort and filter capabilities

✅ **Robust Architecture**
- Async design with Tokio runtime
- Concurrent probe execution
- Thread-safe metrics storage with RwLock
- Graceful shutdown with cancellation tokens
- Comprehensive error handling

### User Interface

**Summary Bar**: System-wide health at a glance
- Overall success rate across all hosts
- Average and P95 latency
- Failure rate per minute

**Host Table**: Status of all monitored hosts
- Color-coded status indicators
- Per-host latency statistics
- Success rate and failure count
- Time since last probe

**Detail Panel**: In-depth information for selected host
- Full connection details
- Detailed latency statistics
- Failure information and patterns
- Recent activity timestamps

**Help Bar**: Always-visible keyboard shortcuts

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate host list |
| `k` / `j` | Navigate (vim-style) |
| `1` - `5` | Jump to specific host |
| `d` | Toggle detail panel |
| `f` | Filter failing hosts |
| `s` | Cycle sort modes |
| `q` | Quit application |

## Technical Highlights

### Performance

- **Memory**: < 10 MB for 5 hosts with 10-minute window
- **CPU**: < 1% idle, < 5% during active probing
- **Network**: Minimal overhead (TCP handshake only)
- **UI Refresh**: 10 FPS (100ms intervals)
- **Binary Size**: 2.1 MB (stripped, optimized)

### Test Coverage

- **92.81% code coverage** (exceeds 85% target)
- **83 tests** across multiple categories:
  - 71 unit tests (core logic validation)
  - 5 integration tests (system-level workflows)
  - 6 network tests (real-world connectivity)
  - 1 documentation test
- **Zero test failures** in continuous integration

### Code Quality

- ✅ All `make ci` checks pass
- ✅ `cargo fmt` - clean formatting
- ✅ `cargo clippy` - zero linter warnings
- ✅ Zero compiler warnings in release build
- ✅ Comprehensive rustdoc documentation
- ✅ Dependabot configured for dependency updates
- ✅ Secret scanning with gitleaks

## Architecture

netglance uses a clean async architecture with separated concerns:

```
┌─────────────────┐
│ Probe Scheduler │ ← Spawns probes at fixed intervals
└────────┬────────┘
         ↓
┌─────────────────┐
│  TCP Workers    │ ← Async TCP connection attempts
└────────┬────────┘
         ↓
┌─────────────────┐
│ Metrics Updater │ ← Processes probe results
└────────┬────────┘
         ↓
┌─────────────────┐
│ Metrics Store   │ ← Rolling window storage
└────────┬────────┘
         ↓
┌─────────────────┐
│  TUI Renderer   │ ← Terminal UI (10 FPS)
└─────────────────┘
```

### Key Technologies

- **Tokio** - Async runtime for concurrent operations
- **Ratatui** - Terminal UI framework
- **Crossterm** - Cross-platform terminal control
- **Clap** - CLI argument parsing
- **Tracing** - Structured logging

## Installation

```bash
# Clone the repository
git clone https://github.com/nmarks/netglance.git
cd netglance

# Build release binary
cargo build --release

# Binary at: ./target/release/netglance
```

## Quick Start

Monitor a single host:
```bash
./target/release/netglance "Google DNS=8.8.8.8:53"
```

Monitor multiple hosts:
```bash
./target/release/netglance \
  "Google DNS=8.8.8.8:53" \
  "Cloudflare=1.1.1.1:53" \
  "Google HTTPS=www.google.com:443"
```

With custom settings:
```bash
./target/release/netglance \
  --interval 2 \
  --timeout 10 \
  --window 15 \
  --log-level debug \
  "API=api.example.com:443"
```

## Documentation

- **README.md** - Comprehensive user guide (400+ lines)
- **USAGE.md** - Detailed usage documentation (600+ lines)
- **Rustdoc** - Generated API documentation (`cargo doc`)
- **PLAN.md** - Implementation plan and progress tracking

## Testing

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Network tests (requires internet)
cargo test --test network_tests

# Coverage report
cargo tarpaulin --lib
```

### Test Categories

**Unit Tests (71 tests)**:
- Configuration validation
- Ring buffer behavior
- Statistics calculation (percentiles, averages)
- Host state management
- Status classification
- Metrics aggregation
- Probe result handling
- TCP worker error mapping

**Integration Tests (5 tests)**:
- End-to-end monitoring workflow
- Multi-host with different states
- Metrics store + updater interaction
- Graceful shutdown verification
- Real network integration with Google

**Network Tests (6 tests)**:
- Real HTTPS connections
- Connection timeout behavior
- Invalid hostname handling
- Connection refused detection
- Sequential probe reliability
- Concurrent probe handling

## Known Limitations

- Maximum of 5 hosts per instance (by design)
- Minimum terminal size: 80x24
- No persistent storage (in-memory only)
- TCP connections only (no UDP or ICMP)
- IPv4 focus (IPv6 not explicitly tested)

## Future Enhancements

Potential features for future releases:

- Configuration file support
- Historical data persistence (SQLite)
- Export metrics to CSV/JSON
- HTTP/HTTPS endpoint monitoring
- Configurable alert thresholds
- Mouse support in TUI
- Sparkline graphs for latency trends
- Multiple monitoring profiles
- Prometheus metrics export

## Credits

**Author**: Nate Marks

**Built With**:
- [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [Crossterm](https://github.com/crossterm-rs/crossterm) - Terminal control
- [Tokio](https://tokio.rs/) - Async runtime

## License

MIT License - See LICENSE file for details

## Support

- Report issues: [GitHub Issues](https://github.com/nmarks/netglance/issues)
- Documentation: [GitHub Repository](https://github.com/nmarks/netglance)

---

**Release Verification**:
- ✅ All 83 tests passing
- ✅ 92.81% test coverage
- ✅ `make ci` passes
- ✅ Release binary builds successfully
- ✅ Zero warnings in release build
- ✅ Documentation complete and verified
- ✅ Real-world network validation complete

**Ready for production use!**
