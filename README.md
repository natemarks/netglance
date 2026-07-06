# netglance

A high-performance TCP connection monitoring tool with a beautiful terminal user interface (TUI). Monitor up to 5 hosts simultaneously with real-time latency tracking, failure detection, and rolling statistics.

![Version](https://img.shields.io/badge/version-0.2.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)
![Tests](https://img.shields.io/badge/tests-126%20passing-brightgreen)

## Features

- 🚀 **Real-time monitoring** - Track TCP connection health with configurable probe intervals
- 📊 **Rolling statistics** - 10-minute rolling window with avg, min, max, and P95 latency
- 🎨 **Beautiful TUI** - Clean interface built with Ratatui and Crossterm
- 🔍 **Detailed diagnostics** - Per-host failure tracking and status indicators
- ⚡ **Async & efficient** - Built on Tokio for high-performance concurrent probing
- 🎯 **Simple controls** - Keyboard-driven interface with intuitive navigation
- 📝 **Comprehensive logging** - Configurable log levels with file output

## Requirements

### Linux
- Any modern Linux distribution (kernel 3.2+)
- No specific GLIBC version required (statically-linked binary)
- Tested on: Ubuntu 20.04+, Debian 11+, CentOS 7+, Alpine Linux

### macOS
- Apple Silicon (M1/M2/M3/M4)
- macOS 11.0 (Big Sur) or later

### For Building from Source
- Rust 1.70 or later
- Cargo

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/nmarks/netglance.git
cd netglance

# Build release binary
cargo build --release

# Binary will be at: ./target/release/netglance

# Optional: Install to system
cargo install --path .
```

### Basic Usage

#### Configuration File (Required)

```bash
# Create default configuration
netglance --init-config

# Edit configuration file
vim ~/.netglance/settings.json

# Start monitoring
netglance
```

**Note:** As of v0.2.0, netglance uses configuration files only. CLI host arguments and setting overrides have been removed for simplicity and consistency.

## Configuration

### Configuration File

netglance supports JSON configuration files for persistent settings.

**Location**: `$HOME/.netglance/settings.json`

**Create**: `netglance --init-config`

### Configuration Structure

**Example configuration**:
```json
{
  "version": "1.0",
  "probe": {
    "interval_seconds": 1,
    "timeout_seconds": 5,
    "window_minutes": 10
  },
  "logging": {
    "level": "info"
  },
  "hosts": [
    {
      "name": "Google DNS",
      "address": "8.8.8.8",
      "port": 53,
      "enabled": true
    },
    {
      "name": "Production API",
      "address": "api.example.com",
      "port": 443,
      "enabled": true
    }
  ]
}
```

**Benefits**:
- No need to type hosts every time
- Easy to version control and share
- Enable/disable hosts without removing them
- Persistent configuration across sessions

### Log Files

Logs are written to: `$HOME/.netglance/logs/netglance.log`

The UI shows status colors only - check logs for detailed error messages.

```bash
# View logs in real-time
tail -f ~/.netglance/logs/netglance.log

# Or use less
less +F ~/.netglance/logs/netglance.log
```

### Configuration Options

**Probe settings:**
- `interval_seconds` (1-60): How often to probe each host
- `timeout_seconds`: Connection timeout for each probe
- `window_minutes`: Rolling window for statistics (default: 10)

**Logging:**
- `level`: Log verbosity (error, warn, info, debug, trace)

**Hosts:**
- `name`: Display name for the host
- `address`: Hostname or IP address
- `port`: TCP port number (1-65535)
- `enabled`: Whether to monitor this host

### Validation

Configuration is validated on startup with helpful error messages:

```bash
$ netglance
Error: Configuration validation failed for: /home/user/.netglance/settings.json

Caused by:
    Invalid probe interval: 0s
    Must be between 1 and 60 seconds.
    
    Edit your config file and set probe.interval_seconds to a value like 5.
```

## Command-Line Options

```
Usage: netglance [OPTIONS]

Options:
  -c, --config <FILE>  Path to config file [default: ~/.netglance/settings.json]
      --init-config    Generate default configuration file and exit
  -h, --help           Print help
  -V, --version        Print version
```

## User Interface

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ TCP Monitor                                                     10m window   │
│ overall: 99.2% success   avg: 18.4 ms   p95: 42 ms   fails: 3/min           │
├──────────────────────────────────────────────────────────────────────────────┤
│ Host            Status   Avg ms   Min   Max   Success   Fail   Last check   │
│ api-1            ● OK      12.3     9     28    100%      0      2s ago     │
│ api-2            ● OK      21.7    14     55     98%      2      1s ago     │
│ db-1             ● WARN    47.9    31    120     95%      5      4s ago     │
│ cache-1          ● OK       6.1     4     14    100%      0      3s ago     │
│ backup-1         ● DOWN      --     --     --     60%     24      8s ago    │
├──────────────────────────────────────────────────────────────────────────────┤
│ Selected host: db-1                                                          │
│ Status: WARN  Address: db.example.com:5432                                   │
│ Avg: 47.9ms  Min: 31ms  Max: 120ms  P95: 88ms                              │
│ Success: 95.0%  Failures: 5  Consecutive: 0                                 │
│ Last success: 3s ago  Last check: 4s ago                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│ [↑↓/k/j] navigate  [1-5] select  [d] details  [f] filter  [s] sort  [q] quit│
└──────────────────────────────────────────────────────────────────────────────┘
```

### Layout Sections

1. **Summary Bar** (top) - Overall system health at a glance
   - Overall success rate across all hosts
   - Average latency across all hosts
   - P95 latency (95th percentile)
   - Failure rate per minute

2. **Host Table** (middle) - Status of all monitored hosts
   - Color-coded status indicators (●)
   - Per-host latency statistics
   - Success rate and failure count
   - Time since last probe

3. **Detail Panel** (bottom, toggleable) - Selected host details
   - Full address and status
   - Detailed latency statistics
   - Failure information
   - Recent activity timestamps

4. **Help Bar** (bottom) - Available keyboard shortcuts

### Status Indicators

- 🟢 **OK (Green)** - Host is healthy (>95% success rate, <3 consecutive failures)
- 🟡 **WARN (Yellow)** - Host is degraded (60-95% success rate or 1-2 consecutive failures)
- 🔴 **DOWN (Red)** - Host is down (>3 consecutive failures or <60% success rate)
- ⚪ **STALE (Gray)** - No recent probe data (>30 seconds old)

## Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate up/down in host list |
| `k` / `j` | Navigate up/down (vim-style) |
| `1` - `5` | Jump directly to host 1-5 |
| `d` | Toggle detail panel on/off |
| `f` | Filter: show only failing hosts |
| `s` | Cycle sort modes (name/latency/failures) |
| `q` | Quit application |
| `Ctrl+C` | Force quit |

## Understanding the Metrics

### Latency Measurements

- **Avg** - Average latency over the rolling window (e.g., last 10 minutes)
- **Min** - Minimum observed latency in the window
- **Max** - Maximum observed latency in the window
- **P95** - 95th percentile latency (95% of probes were faster than this)

### Success Rate

Percentage of successful TCP connections over the rolling window. A connection is successful if:
- TCP handshake completes within the timeout
- No connection refused or network errors occur

### Failure Count

Total number of failed connection attempts in the rolling window. Failures include:
- **Timeout** - Connection attempt exceeded timeout duration
- **Connection Refused** - Remote host actively refused connection
- **DNS Failure** - Unable to resolve hostname
- **Network Error** - Other network-level failures

### Consecutive Failures

Number of failed probes in a row. High consecutive failures trigger the DOWN status. Counter resets on any successful probe.

### Debugging

Control log verbosity via environment variable:
```bash
# Basic info logging (default)
netglance

# Debug logging for troubleshooting
RUST_LOG=debug netglance

# Available levels: error, warn, info, debug, trace
```

Logs are written to: `$HOME/.netglance/logs/netglance.log`

## Architecture

For detailed architecture documentation, see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

**High-level overview:**

```
ProbeScheduler → MetricsUpdater → MetricsStore
                                        ↓
                                   TUI (App)
```

- **ProbeScheduler**: Sends TCP probes on interval
- **MetricsUpdater**: Processes results via channel
- **MetricsStore**: Maintains rolling window of metrics
- **TUI**: Renders real-time UI at 10 FPS

### Key Technologies

- **Tokio** - Async runtime for concurrent operations
- **Ratatui** - Terminal UI framework
- **Crossterm** - Cross-platform terminal control
- **Serde** - JSON configuration serialization

## Development

For detailed development guide, see [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).

### Quick Start

```bash
# Run tests
make test

# Run static checks
make static

# Build debug
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- "Google=8.8.8.8:53"
```

### Testing

```bash
# Run all tests (83 tests)
cargo test

# Run only unit tests (71 tests)
cargo test --lib

# Run integration tests (5 tests)
cargo test --test integration_tests

# Run network tests (6 tests)
cargo test --test network_tests

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Full CI checks (includes formatting, linting, tests)
make ci
```

## Troubleshooting

### Terminal Size

Minimum terminal size: **80x24**. If your terminal is too small, the UI may not render correctly.

### Network Issues

**DNS resolution fails**: Ensure you have network connectivity and the hostname is correct.

**Connection timeouts**: Increase timeout in config file:
```json
{
  "probe": {
    "timeout_seconds": 10
  }
}
```

**High failure rates**: Check if:
- The target host is actually reachable
- Firewall rules aren't blocking connections
- The port number is correct

### Log File Location

Logs are written to `$HOME/.netglance/logs/netglance.log`. Check this file for detailed error messages:
```bash
tail -f ~/.netglance/logs/netglance.log
```

## Performance

netglance is designed for efficiency:

- **Memory usage**: < 10 MB for 5 hosts with 10-minute windows
- **CPU usage**: < 1% idle, < 5% during active probing
- **Network**: Minimal overhead (TCP handshake only, no data transfer)

### Benchmarks

On a modern system monitoring 5 hosts with 1-second intervals:
- Probe latency: < 1ms overhead
- UI refresh rate: 10 FPS (100ms)
- Statistics calculation: < 10µs per host

### Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --lib

# Run CI checks (formatting, linting, tests)
make ci
```

### Making a Release

```bash
# Preview what would happen
make dry-run-release VERSION=v0.2.1

# Create GitHub release (requires gh CLI)
make github-release VERSION=v0.2.1
```

**Requirements for releases**:
- GitHub CLI (`gh`) installed and authenticated
- Clean git working directory
- All tests passing

The release process will:
1. Run CI checks
2. Update version in Cargo.toml
3. Build release binary
4. Create git commit and tag
5. Push to GitHub
6. Create GitHub release with binary

## Security & Quality

netglance uses pre-commit hooks to maintain code quality and security:

- **Gitleaks:** Scans for secrets (API keys, tokens, credentials)
- **Static Analysis:** Format, lint, tests, security audit
- **GitHub Actions:** Runs same checks on all PRs
- **Configured via:** `.gitleaks.toml`, `Makefile`

**Setup:**
```bash
# Install gitleaks
brew install gitleaks  # macOS
# or see: https://github.com/gitleaks/gitleaks#installing

# Install pre-commit hook (runs gitleaks + make static)
make install-hooks
```

See [`docs/SETUP.md`](docs/SETUP.md) for detailed setup.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. **Install pre-commit hooks** (`make install-hooks`)
4. Make your changes
5. Run tests and checks (`make ci`)
6. Commit changes (`git commit -m 'Add amazing feature'`)
7. Push to branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for detailed development guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- Terminal control via [Crossterm](https://github.com/crossterm-rs/crossterm)
- Async runtime powered by [Tokio](https://tokio.rs/)

## Documentation

- 📖 [Architecture Overview](docs/ARCHITECTURE.md) - System design and key decisions
- 🔧 [Development Guide](docs/DEVELOPMENT.md) - How to contribute and extend
- 🚀 [Setup Guide](docs/SETUP.md) - Development environment setup
- 📦 [Release Guide](docs/RELEASE_GUIDE.md) - Complete release process with examples
- 📝 [Release Notes](RELEASE_NOTES.md) - Version history and changes

## Support

- 📫 Report issues: [GitHub Issues](https://github.com/nmarks/netglance/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/nmarks/netglance/discussions)

---

**Version 0.2.0** | Made by Nate Marks
