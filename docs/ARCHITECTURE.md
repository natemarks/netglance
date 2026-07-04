# Architecture Overview

**netglance** is a Rust-based TCP connection monitoring tool with a terminal user interface (TUI). It continuously probes TCP endpoints, tracks latency and reliability metrics, and displays real-time health status.

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                      main.rs                            │
│  ┌───────────────────────────────────────────────────┐ │
│  │ • Parse config file                               │ │
│  │ • Initialize components                           │ │
│  │ • Wire async tasks with channels                  │ │
│  │ • Manage application lifecycle                    │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ProbeScheduler│  │MetricsUpdater│  │   TUI (App)  │
│              │  │              │  │              │
│ • DNS lookup │  │ • Channels   │  │ • Render UI  │
│ • TCP probes │  │ • Aggregate  │  │ • Handle keys│
│ • Intervals  │  │ • Update     │  │ • Events     │
└──────────────┘  └──────────────┘  └──────────────┘
        │                 │                 │
        │ ProbeResult     │                 │
        └────────────────►│                 │
                          │                 │
                          │   Read          │
                          └────────────────►│
                                            │
                          ┌─────────────────┘
                          ▼
                  ┌──────────────┐
                  │ MetricsStore │
                  │              │
                  │ • HostState  │
                  │ • Statistics │
                  │ • Snapshots  │
                  └──────────────┘
```

### Data Flow

1. **Configuration Loading**
   - `main.rs` loads JSON config from `~/.netglance/settings.json`
   - Config specifies hosts, probe interval, timeout, rolling window size
   - Validation ensures configuration is sensible

2. **Probe Scheduling**
   - `ProbeScheduler` resolves DNS names to IP addresses
   - Initiates TCP connections on configured interval
   - Each probe measures connection latency or records failure
   - Results sent via unbounded MPSC channel

3. **Metrics Processing**
   - `MetricsUpdater` receives probe results from channel
   - Updates `MetricsStore` with new data points
   - Calculates rolling statistics (success rate, latency percentiles)
   - Logs snapshots to JSONL file every 10 seconds

4. **UI Rendering**
   - TUI reads `MetricsStore` for current state
   - Renders table with all hosts and their metrics
   - Detail view for selected host with sparkline
   - Updates at 10 FPS for smooth user experience

5. **Graceful Shutdown**
   - `CancellationToken` signals all tasks to stop
   - Background tasks drain channels and exit cleanly
   - Terminal is restored to original state

## Module Structure

```
src/
├── main.rs              # Application entry point and orchestration
├── lib.rs               # Public API re-exports
├── config/              # Configuration management
│   ├── cli.rs           # ProbeConfig internal type
│   ├── directory.rs     # Config file paths
│   ├── schema.rs        # JSON config structure
│   └── validation.rs    # Config validation with helpful errors
├── constants.rs         # Global constants
├── logging.rs           # Log file management and rotation
├── metrics/             # Data structures and statistics
│   ├── host_state.rs    # Per-host metrics tracking
│   ├── overall.rs       # Aggregate metrics across hosts
│   ├── ring_buffer.rs   # Time-windowed sample storage
│   ├── statistics.rs    # Statistical calculations
│   ├── status.rs        # Health status classification
│   ├── store.rs         # Central metrics repository
│   └── updater.rs       # Background metrics processor
├── probe/               # TCP probing infrastructure
│   ├── dns.rs           # DNS resolution
│   ├── scheduler.rs     # Probe scheduling and orchestration
│   └── worker.rs        # TCP connection worker
└── ui/                  # Terminal user interface
    ├── app.rs           # Application state and event handling
    ├── render.rs        # Main render loop
    ├── terminal.rs      # Terminal setup/restore
    └── widgets/         # UI components
        ├── details.rs   # Host detail panel with sparkline
        ├── footer.rs    # Status bar and help text
        ├── host_panel.rs# Compact host overview panels
        └── table.rs     # Host list table view
```

### Key Design Decisions

#### 1. Why Arc<RwLock<MetricsStore>>?

**Problem:** Multiple async tasks need concurrent access to metrics.

**Solution:** `Arc<RwLock<T>>` provides:
- **Arc**: Shared ownership across tasks
- **RwLock**: Multiple readers, single writer
- **Read-heavy workload**: UI reads constantly, updater writes periodically

**Alternative considered:** Actor pattern with message passing
**Reason for rejection:** Added complexity without clear benefit for this use case

#### 2. Why Unbounded Channels?

**Problem:** Probe results need to flow from scheduler to updater.

**Solution:** Unbounded MPSC channel because:
- Probes are rate-limited by interval (max 1/sec per host)
- Updater processes results very quickly
- Backpressure unlikely in normal operation
- Simplicity over bounded channel complexity

**Trade-off:** Potential memory growth if updater stops processing
**Mitigation:** CancellationToken ensures clean shutdown

#### 3. Why No Retry Logic?

**Problem:** Early versions had complex initialization retry system.

**Decision:** Removed in favor of continuous probing because:
- Regular probing naturally handles transient failures
- Failed probe = data point showing host is down
- Eventual recovery when host comes back online
- Simpler code, easier to reason about

**Result:** Removed 270 lines of retry infrastructure

#### 4. Why Config File Only (No CLI Overrides)?

**Problem:** Multiple configuration sources created confusion.

**Decision:** Config file only because:
- Single source of truth
- Easier to reproduce issues
- Forces users to understand configuration
- Simpler code (removed 80 lines)

**Trade-off:** Less convenient for quick tests
**Mitigation:** Easy to edit config file, templates provided

#### 5. Why RingBuffer Instead of Full History?

**Problem:** Storing all probe results consumes unbounded memory.

**Solution:** Fixed-size ring buffer per host:
- Rolling 10-minute window (600 samples at 1/sec)
- Old samples automatically discarded
- Bounded memory usage: O(hosts × window_size)
- Recent data always available

**Memory footprint:** ~100 bytes × 600 samples × 5 hosts = ~300 KB

## Async Architecture

### Task Orchestration

The application uses Tokio for async runtime:

```rust
// Three independent background tasks
tokio::spawn(probe_scheduler.run());    // Probing loop
tokio::spawn(metrics_updater.run());    // Update loop
tokio::spawn(event_handler.run());      // Event loop

// Main loop: UI rendering + event handling
loop {
    select! {
        _ = metrics_log_interval.tick() => log_metrics(),
        event = event_receiver.recv() => handle_event(event),
    }
}
```

**Key characteristics:**
- Tasks are independent and loosely coupled
- Communication via channels (MPSC for data, broadcast for control)
- CancellationToken for graceful shutdown
- No shared mutable state except MetricsStore (protected by RwLock)

### Concurrency Model

```
Thread Pool (Tokio runtime)
├── ProbeScheduler task
│   └── Spawns DNS resolution per host (tokio::spawn)
├── MetricsUpdater task
│   └── Processes results sequentially
├── EventHandler task
│   └── Polls terminal events
└── Main task (UI)
    └── Renders frames, handles events
```

**Safety guarantees:**
- No data races (RwLock enforces single writer)
- No deadlocks (simple lock hierarchy)
- Cancellation-safe (tasks check token regularly)

## State Management

### MetricsStore Structure

```rust
MetricsStore
├── hosts: Vec<HostState>           // One per monitored host
│   └── HostState
│       ├── metadata: (name, addr)
│       ├── samples: RingBuffer     // Time-windowed probe results
│       ├── metrics: HostMetrics    // Cached statistics
│       └── status: HostStatus      // Health classification
└── config: ProbeConfig             // Probe timing settings
```

### Snapshot Pattern

UI reads via immutable snapshots:
```rust
let snapshot = metrics_store.snapshot(Instant::now());
// snapshot is independent, can be used without holding lock
render_ui(snapshot);
```

**Benefits:**
- Lock held for minimal time
- UI rendering can't block metrics updates
- Snapshot is consistent point-in-time view

## Performance Characteristics

### Probe Latency

- **DNS resolution:** Cached after first lookup, ~5-50ms
- **TCP connect:** Depends on network, typically 1-100ms
- **Overhead:** <1ms per probe
- **Concurrency:** All hosts probed in parallel

### Memory Usage

- **Base:** ~5 MB (binary + runtime)
- **Per host:** ~60 KB (ring buffer + metadata)
- **5 hosts @ 10 min window:** ~300 KB
- **Total typical:** ~6 MB resident

### CPU Usage

- **Idle:** <1% (waiting on timers)
- **Probing:** <5% (network I/O bound)
- **UI rendering:** <3% (60 FPS cap)
- **Total typical:** <10% CPU

## Error Handling Strategy

### Error Types

```rust
// Probe errors (recoverable)
ProbeError {
    Timeout,         // Connection took too long
    ConnectionRefused, // Host actively refused
    DnsFailure,      // Hostname doesn't resolve
}

// Application errors (fatal)
anyhow::Error       // Config, I/O, initialization
```

**Philosophy:**
- Probe failures are **data points**, not errors
- Application failures should **fail fast** with context
- User-facing errors must be **actionable**

### Validation Strategy

**Config validation:**
1. JSON parsing (serde)
2. Schema validation (ConfigValidator)
3. Runtime checks (DNS resolution)

**Example error:**
```
Invalid probe interval: 0s
Must be between 1 and 60 seconds.

Edit your config file and set probe.interval_seconds to a value like 5.
```

## Testing Strategy

### Unit Tests (106 tests)

- **config:** Validation, parsing, defaults
- **metrics:** Statistics, ring buffer, status classification
- **probe:** DNS resolution, error mapping
- **Isolated:** No network, fast (<1s)

### Integration Tests (5 tests)

- **End-to-end:** All components wired together
- **Real TCP:** Local test server (TestServer helper)
- **Lifecycle:** Startup, operation, graceful shutdown
- **Fast:** ~2.5s total

### Network Tests (6 tests)

- **Real internet:** Test against www.google.com:443
- **DNS failures:** Invalid hostnames
- **Timeouts:** Very short timeout values
- **Concurrent probes:** Parallel execution
- **Slow:** ~5s total (network latency)

### Test Helpers

```rust
TestServer::start()  // RAII pattern, auto-cleanup
// Binds random port, accepts connections, drops on Drop
```

**Benefits:**
- No manual cleanup
- No port conflicts
- Explicit resource management

## Deployment Considerations

### Single Binary

- **Statically linked:** No runtime dependencies
- **Cross-platform:** Linux, macOS, Windows
- **Size:** ~8 MB (release build with optimizations)

### Configuration

- **Location:** `~/.netglance/settings.json`
- **Format:** JSON (human-editable)
- **Validation:** Helpful errors on startup
- **Example:** Generated via `--init-config`

### Logging

- **Location:** `~/.netglance/logs/`
- **Format:** JSONL (one event per line)
- **Rotation:** Automatic compression after 24 hours
- **Content:** Periodic metric snapshots

## Extension Points

### Adding New Probe Types

Current: TCP connection
Future: HTTP, ICMP, custom protocols

**Steps:**
1. Create new module in `probe/`
2. Implement probe function: `async fn probe_X(target, timeout) -> ProbeResult`
3. Add protocol field to HostConfig
4. Update ProbeScheduler to dispatch by protocol

### Adding New Metrics

Current: Latency, success rate, consecutive failures
Future: Jitter, packet loss, custom thresholds

**Steps:**
1. Update HostMetrics struct in `metrics/statistics.rs`
2. Add calculation in `calculate_metrics()`
3. Update UI widgets to display new metrics
4. Update tests

### Adding New UI Views

Current: Table, detail panel
Future: Graph view, multi-host comparison, alerts

**Steps:**
1. Create widget in `ui/widgets/`
2. Implement rendering logic
3. Add keyboard shortcut in `app.rs`
4. Update help text in footer

## Known Limitations

1. **TCP only:** No HTTP, ICMP, or UDP support
2. **IPv4 primary:** IPv6 falls back, not preferred
3. **Single config:** Can't monitor different host groups
4. **No alerts:** Just displays, doesn't notify
5. **No API:** Terminal UI only, no programmatic access

## Future Considerations

### Potential Enhancements

- **Multi-protocol:** Support HTTP health checks
- **Alerting:** Desktop notifications on status change
- **Export:** Prometheus metrics endpoint
- **Dashboards:** Web-based dashboard (in addition to TUI)
- **Distributed:** Agent mode for remote monitoring

### Scalability Notes

Current design supports:
- **Hosts:** 5 (UI optimized for single screen)
- **Probe interval:** 1-60 seconds
- **Window:** 10 minutes (600 samples)

For larger deployments:
- Consider agent/server architecture
- Database backend for metrics storage
- Web dashboard for multi-host visualization

---

**Last Updated:** 2026-07-04  
**Version:** 0.2.0  
**Author:** Nate Marks
