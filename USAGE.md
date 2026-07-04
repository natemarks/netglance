# netglance - Detailed Usage Guide

This guide provides comprehensive information on using netglance effectively for TCP connection monitoring.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Understanding the Interface](#understanding-the-interface)
3. [Interpreting Metrics](#interpreting-metrics)
4. [Common Use Cases](#common-use-cases)
5. [Advanced Configuration](#advanced-configuration)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

## Getting Started

### Your First Monitoring Session

The simplest way to start is monitoring a single reliable host:

```bash
./netglance "Google DNS=8.8.8.8:53"
```

This will:
- Connect to Google's DNS server on port 53
- Probe every 1 second (default interval)
- Track statistics over a 10-minute window (default)
- Display results in real-time

### Monitoring Multiple Services

For a typical web application stack:

```bash
./netglance \
  "Web Server=web.example.com:443" \
  "API Gateway=api.example.com:443" \
  "Database=db.internal:5432" \
  "Cache=redis.internal:6379" \
  "Queue=rabbitmq.internal:5672"
```

This monitors all five hosts simultaneously, showing you which component is experiencing issues.

## Understanding the Interface

### Section 1: Summary Bar

```
TCP Monitor                                                     10m window
overall: 99.2% success   avg: 18.4 ms   p95: 42 ms   fails: 3/min
```

**Fields:**
- **Window**: Time range for statistics (e.g., "10m window" = last 10 minutes)
- **Overall success**: Combined success rate across all hosts
- **avg**: Average latency across all successful probes from all hosts
- **p95**: 95th percentile latency (slower probes ignored for noise reduction)
- **fails**: Number of failures per minute across all hosts

**What to watch:**
- Success < 99%: Some hosts experiencing issues
- Success < 95%: Significant problems
- High p95 (>2x avg): Intermittent latency spikes
- Rising fail rate: Degrading connections

### Section 2: Host Table

```
Host            Status   Avg ms   Min   Max   Success   Fail   Last check
api-1            ● OK      12.3     9     28    100%      0      2s ago
api-2            ● OK      21.7    14     55     98%      2      1s ago
db-1             ● WARN    47.9    31    120     95%      5      4s ago
```

**Columns explained:**

1. **Host**: Your friendly name for the endpoint
2. **Status**: Visual health indicator (see Status Guide below)
3. **Avg ms**: Mean latency over the rolling window
4. **Min**: Fastest connection in the window
5. **Max**: Slowest connection in the window
6. **Success**: Percentage of successful probes
7. **Fail**: Total failed probes in window
8. **Last check**: Time since most recent probe

**Status Guide:**

| Symbol | Status | Meaning | Trigger |
|--------|--------|---------|---------|
| 🟢 ● | OK | Healthy | >95% success, <3 consecutive failures |
| 🟡 ● | WARN | Degraded | 60-95% success OR 1-2 consecutive failures |
| 🔴 ● | DOWN | Failing | <60% success OR 3+ consecutive failures |
| ⚪ ● | STALE | No data | No probe in >30 seconds |

**Reading patterns:**

- **Increasing Max**: Latency spikes occurring
- **High Fail + WARN**: Intermittent connectivity issues
- **100% Success + High Avg**: Slow but stable connection
- **Stale status**: Probing may be stuck or cancelled

### Section 3: Detail Panel (press 'd' to toggle)

```
Selected host: db-1
Status: WARN  Address: db.example.com:5432
Avg: 47.9ms  Min: 31ms  Max: 120ms  P95: 88ms
Success: 95.0%  Failures: 5  Consecutive: 0
Last success: 3s ago  Last check: 4s ago
```

**Additional information:**

- **Address**: Full connection string being probed
- **P95**: 95th percentile latency (useful for SLA monitoring)
- **Consecutive**: Failed probes in a row (triggers DOWN status)
- **Last success**: When the last successful connection occurred
- **Last check**: When the most recent probe attempt finished

**Why these matter:**

- **P95 vs Avg**: Large gap indicates inconsistent latency
- **Consecutive failures**: Pattern of ongoing issues vs intermittent
- **Last success age**: How long the connection has been down

### Section 4: Help Bar

```
[↑↓/k/j] navigate  [1-5] select  [d] details  [f] filter  [s] sort  [q] quit
```

Always visible reference for keyboard controls.

## Interpreting Metrics

### Latency Analysis

**What is "good" latency?**

- **< 10ms**: Excellent (local network, nearby servers)
- **10-50ms**: Good (same region, well-connected)
- **50-100ms**: Acceptable (cross-region, some distance)
- **100-200ms**: Slow (distant or congested)
- **> 200ms**: Poor (investigate routing or server load)

**Patterns to watch:**

1. **Steadily increasing latency**: Server load growing or network congestion
2. **Spiky latency (high max, low min)**: Intermittent issues, possibly TCP retransmits
3. **Bimodal latency**: Two different paths (load balancer issue?)
4. **Consistent high latency**: Distance, slow server, or bandwidth constraint

### Success Rate Interpretation

**Success rate thresholds:**

- **100%**: Perfect reliability (rare in production)
- **99.9%**: Excellent (3 failures per 1000 probes)
- **99%**: Good (10 failures per 1000 probes)
- **95-99%**: Marginal (investigate if sustained)
- **< 95%**: Poor (immediate attention needed)

**Calculating downtime from success rate:**

With 1-second probes over 10 minutes (600 probes):
- 99.9% success = 0.6 failures = ~1 second of downtime
- 99% success = 6 failures = ~6 seconds of downtime
- 95% success = 30 failures = ~30 seconds of downtime

### Failure Patterns

**Types of failures:**

1. **Timeout**: Connection attempt exceeded timeout duration
   - Causes: Firewall dropping packets, server overloaded, network congestion
   - Action: Check server load, firewall rules, increase timeout if needed

2. **Connection Refused**: Server actively rejected connection
   - Causes: Service not running, wrong port, host firewall
   - Action: Verify service is running, check port number, review firewall rules

3. **DNS Failure**: Unable to resolve hostname
   - Causes: DNS server down, wrong hostname, network issue
   - Action: Verify hostname, check DNS configuration, try IP address

4. **Network Error**: Other network-level failures
   - Causes: Network unreachable, interface down, routing issues
   - Action: Check network connectivity, routing tables, interface status

**Consecutive failures:**

- **1-2**: Could be transient (network blip, server restart)
- **3+**: Sustained outage (triggers DOWN status)
- **10+**: Serious problem requiring immediate investigation

## Common Use Cases

### Monitoring Web Services

```bash
./netglance \
  "Production API=api.example.com:443" \
  "Staging API=staging-api.example.com:443" \
  "Dev API=dev-api.example.com:443"
```

**What to watch:**
- Compare latencies across environments
- Identify which environment is experiencing issues
- Detect deployment problems (sudden latency changes)

### Database Health Monitoring

```bash
./netglance \
  --interval 2 \
  --timeout 10 \
  "Primary DB=db-primary.internal:5432" \
  "Replica 1=db-replica-1.internal:5432" \
  "Replica 2=db-replica-2.internal:5432"
```

**What to watch:**
- Replica lag (higher latency on replicas)
- Failover events (primary becomes WARN/DOWN)
- Connection pool exhaustion (timeouts increase)

### Load Balancer Backend Check

```bash
./netglance \
  "LB Frontend=lb.example.com:443" \
  "Backend 1=backend-1.internal:8080" \
  "Backend 2=backend-2.internal:8080" \
  "Backend 3=backend-3.internal:8080"
```

**What to watch:**
- All backends have similar latency
- No single backend showing high failures
- Frontend latency <= backend latencies

### Network Path Analysis

```bash
./netglance \
  --interval 5 \
  --window 30 \
  "Local=192.168.1.1:80" \
  "Gateway=10.0.0.1:80" \
  "ISP DNS=8.8.8.8:53" \
  "Remote=remote.example.com:443"
```

**What to watch:**
- Identify where latency increases in path
- Detect network segment issues
- Measure end-to-end connectivity

## Advanced Configuration

### Tuning Probe Timing

**Fast monitoring (every 0.5 seconds):**
```bash
./netglance --interval 0.5 --timeout 2 "Host=example.com:443"
```
- Use for: Critical services, short-duration testing
- Trade-off: Higher network traffic, more CPU usage

**Slow monitoring (every 10 seconds):**
```bash
./netglance --interval 10 --timeout 5 "Host=example.com:443"
```
- Use for: Long-term monitoring, reduced load
- Trade-off: Slower problem detection

**Rule of thumb:** Timeout should be < Interval to avoid queue buildup

### Adjusting Window Duration

**Short window (5 minutes):**
```bash
./netglance --window 5 "Host=example.com:443"
```
- Use for: Rapid changes, quick baseline establishment
- Trade-off: Less historical context, more volatile metrics

**Long window (60 minutes):**
```bash
./netglance --window 60 "Host=example.com:443"
```
- Use for: Trend analysis, stable baseline, SLA monitoring
- Trade-off: Slower to reflect changes, more memory usage

### Log Level Configuration

**Debug logging for troubleshooting:**
```bash
./netglance --log-level debug "Host=example.com:443"
# Check logs: tail -f netglance.log
```

**Minimal logging for production:**
```bash
./netglance --log-level error "Host=example.com:443"
```

**Environment variable override:**
```bash
RUST_LOG=trace ./netglance "Host=example.com:443"
```

## Best Practices

### Choosing Probe Intervals

1. **Production monitoring**: 1-5 second intervals
2. **Development testing**: 0.5-1 second intervals
3. **Long-term trends**: 10-30 second intervals
4. **Troubleshooting**: 0.5-1 second intervals with short window

### Naming Hosts

Good names help quick identification:

**Good examples:**
- `"Production API=api.example.com:443"`
- `"DB Primary=db-01.internal:5432"`
- `"Cache West=redis-west.internal:6379"`

**Bad examples:**
- `"Host 1=example.com:443"` (unclear purpose)
- `"example.com:443=example.com:443"` (redundant)
- `"x=1.2.3.4:80"` (cryptic)

### Organizing Multiple Sessions

For complex environments, run multiple instances:

```bash
# Terminal 1: Web tier
./netglance "LB=lb.example.com:443" "Web1=web1.internal:80" ...

# Terminal 2: Data tier
./netglance "DB=db.internal:5432" "Cache=redis.internal:6379" ...

# Terminal 3: External dependencies
./netglance "Payment API=api.stripe.com:443" "Email=smtp.sendgrid.net:587" ...
```

### Interpreting Results During Incidents

1. **Sort by failures** (`s` key): Identify worst-affected hosts
2. **Filter failures only** (`f` key): Focus on problem hosts
3. **Check detail panel** (`d` key): Understand failure patterns
4. **Monitor consecutive failures**: Determine if issue is sustained
5. **Compare to baseline**: Use historical knowledge of normal metrics

## Troubleshooting

### Issue: "Connection Timeout" for reachable host

**Symptoms:** High timeout failures, but host is accessible

**Possible causes:**
1. Timeout too short for network conditions
2. Server is slow to accept connections (high load)
3. Network congestion

**Solutions:**
```bash
# Increase timeout
./netglance --timeout 10 "Host=example.com:443"

# Reduce probe frequency
./netglance --interval 5 --timeout 10 "Host=example.com:443"
```

### Issue: All hosts showing "STALE"

**Symptoms:** Gray status indicators, "Last check" times increasing

**Possible causes:**
1. Probing system stuck or cancelled
2. Main thread blocked

**Solutions:**
1. Check logs: `cat netglance.log`
2. Ensure interval > timeout
3. Verify system has network access
4. Restart netglance

### Issue: High latency reported, but manual test is fast

**Symptoms:** netglance shows 100ms+, but `curl` or `ping` show <10ms

**Explanation:** netglance measures TCP handshake time, not just ICMP ping

**Verification:**
```bash
# Test TCP connection time
time nc -zv example.com 443
```

TCP handshake includes:
- DNS resolution
- SYN/ACK exchange
- Connection establishment
- Often slower than ICMP

### Issue: Memory usage grows over time

**Symptoms:** netglance process memory increases

**Possible causes:**
1. Very long window with many samples
2. Memory leak (rare, but possible)

**Solutions:**
```bash
# Reduce window duration
./netglance --window 5 "Host=example.com:443"

# Restart periodically for long-term monitoring
```

Expected memory usage:
- 5 hosts, 10-minute window, 1-second interval: ~5-10 MB
- 5 hosts, 60-minute window, 1-second interval: ~20-30 MB

### Issue: UI doesn't fit in terminal

**Symptoms:** Broken layout, overlapping text

**Solution:** Ensure terminal is at least 80x24:
```bash
# Check current size
echo $COLUMNS x $LINES

# Resize terminal or use smaller font
```

### Issue: Cannot connect to low ports (<1024)

**Symptoms:** Connection refused for ports like 80, 443, 22

**Cause:** Some systems require root for low ports (rare for outbound connections)

**Solution:**
```bash
# Usually not needed for outbound connections
# But if required:
sudo ./netglance "Host=example.com:80"
```

### Issue: DNS resolution is slow

**Symptoms:** First probe very slow, subsequent probes fast

**Explanation:** DNS caching after first lookup

**Mitigation:**
```bash
# Use IP address to bypass DNS
./netglance "Host=1.2.3.4:443"
```

## Tips & Tricks

### Quick Health Check

Press `f` (filter) to see only failing hosts. If screen is empty, everything is healthy!

### Baseline Establishment

Run for one window duration (default 10 minutes) to establish baseline, then start monitoring for anomalies.

### Comparing Environments

Run side-by-side in tmux/screen:
```bash
# Pane 1
./netglance "Prod=prod.example.com:443"

# Pane 2
./netglance "Staging=staging.example.com:443"
```

### Scripted Monitoring

```bash
#!/bin/bash
# Start monitoring in background, kill after 1 hour
timeout 3600 ./netglance "Host=example.com:443" &
NETGLANCE_PID=$!

# Do other work...

# Check if still running
if ps -p $NETGLANCE_PID > /dev/null; then
    kill $NETGLANCE_PID
fi
```

### Integration with Alerts

```bash
# Monitor logs for errors
tail -f netglance.log | grep -i 'error\|fail' | \
  while read line; do
    echo "Alert: $line" | mail -s "netglance alert" admin@example.com
  done
```

---

For more information, see the main [README.md](README.md) or visit the [GitHub repository](https://github.com/nmarks/netglance).
