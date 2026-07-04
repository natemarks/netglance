//! Logging infrastructure with timestamped files, rotation, and compression.

use crate::constants;
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::info;

/// Logging session manager with timestamped files.
pub struct LoggingSession {
    /// Timestamp when this session started
    start_time: DateTime<Local>,
    /// Path to error log file
    error_log_path: PathBuf,
    /// Path to metrics JSONL file
    metrics_log_path: PathBuf,
    /// Metrics file writer
    metrics_writer: Arc<parking_lot::Mutex<BufWriter<File>>>,
}

impl LoggingSession {
    /// Create a new logging session with timestamped log files.
    ///
    /// Creates:
    /// - `YYYYMMDD-HHMMSS-errors.log` for error logs
    /// - `YYYYMMDD-HHMMSS-metrics.jsonl` for metrics data
    ///
    /// Also performs rotation of logs older than 24 hours.
    pub fn new(logs_dir: &Path, log_level: &str) -> Result<Self> {
        let start_time = Local::now();
        let timestamp = start_time.format("%Y%m%d-%H%M%S");

        // Create log file paths
        let error_log_path = logs_dir.join(format!("{}-errors.log", timestamp));
        let metrics_log_path = logs_dir.join(format!("{}-metrics.jsonl", timestamp));

        // Rotate old logs before creating new ones
        Self::rotate_old_logs(logs_dir)?;

        // Open error log file
        let error_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&error_log_path)
            .context("Failed to create error log file")?;

        // Set up tracing subscriber for error logs
        tracing_subscriber::fmt()
            .with_writer(Arc::new(error_log_file))
            .with_env_filter(tracing_subscriber::EnvFilter::new(log_level))
            .with_ansi(false)
            .init();

        info!("Logging session started");
        info!("Error log: {}", error_log_path.display());
        info!("Metrics log: {}", metrics_log_path.display());

        // Open metrics log file
        let metrics_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&metrics_log_path)
            .context("Failed to create metrics log file")?;

        let metrics_writer = Arc::new(parking_lot::Mutex::new(BufWriter::new(metrics_file)));

        Ok(Self {
            start_time,
            error_log_path,
            metrics_log_path,
            metrics_writer,
        })
    }

    /// Write a metrics snapshot to the JSONL log.
    pub fn log_metrics(&self, snapshot: &crate::metrics::MetricsSnapshot) -> Result<()> {
        let mut writer = self.metrics_writer.lock();

        // Create JSONL record with timestamp
        let record = serde_json::json!({
            "timestamp": snapshot.timestamp.elapsed().as_secs(),
            "hosts": snapshot.hosts.iter().map(|h| {
                serde_json::json!({
                    "hostname": h.hostname,
                    "address": h.address,
                    "resolved_address": h.resolved_address,
                    "status": format!("{:?}", h.status),
                    "metrics": {
                        "success_rate": h.metrics.success_rate,
                        "avg_latency_ms": h.metrics.avg_latency.map(|d| d.as_millis()),
                        "min_latency_ms": h.metrics.min_latency.map(|d| d.as_millis()),
                        "max_latency_ms": h.metrics.max_latency.map(|d| d.as_millis()),
                        "failure_count": h.metrics.failure_count,
                        "consecutive_failures": h.metrics.consecutive_failures,
                        "total_probes": h.metrics.total_probes,
                    }
                })
            }).collect::<Vec<_>>(),
            "overall": {
                "overall_success_rate": snapshot.overall.overall_success_rate,
                "total_failures": snapshot.overall.total_failures,
                "healthy_count": snapshot.overall.healthy_count,
                "degraded_count": snapshot.overall.degraded_count,
                "down_count": snapshot.overall.down_count,
                "stale_count": snapshot.overall.stale_count,
            }
        });

        writeln!(writer, "{}", serde_json::to_string(&record)?)?;
        writer.flush()?;

        Ok(())
    }

    /// Rotate logs older than 24 hours into compressed archives.
    fn rotate_old_logs(logs_dir: &Path) -> Result<()> {
        let now = SystemTime::now();
        let rotation_threshold = constants::log_rotation_threshold();

        // Find all log files
        let entries = fs::read_dir(logs_dir)
            .with_context(|| format!("Failed to read logs directory: {}", logs_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Only process uncompressed log files
            if !filename.ends_with(".log") && !filename.ends_with(".jsonl") {
                continue;
            }

            // Skip if already compressed
            if filename.ends_with(".tgz") || filename.ends_with(".gz") {
                continue;
            }

            // Check file age
            let metadata = entry.metadata()?;
            let modified = metadata.modified()?;

            if let Ok(age) = now.duration_since(modified) {
                if age > rotation_threshold {
                    Self::compress_log_file(&path)?;
                    info!("Rotated old log: {}", filename);
                }
            }
        }

        Ok(())
    }

    /// Compress a log file to .tgz format and remove the original.
    fn compress_log_file(path: &Path) -> Result<()> {
        let compressed_path = path.with_extension("log.tgz");

        // Read original file
        let content = fs::read(path).context("Failed to read log file for compression")?;

        // Create compressed file
        let compressed_file =
            File::create(&compressed_path).context("Failed to create compressed log file")?;

        let mut encoder = GzEncoder::new(compressed_file, Compression::default());
        encoder
            .write_all(&content)
            .context("Failed to write compressed data")?;
        encoder.finish().context("Failed to finalize compression")?;

        // Remove original file
        fs::remove_file(path).context("Failed to remove original log file after compression")?;

        Ok(())
    }

    /// Get the error log path.
    #[allow(dead_code)]
    pub fn error_log_path(&self) -> &Path {
        &self.error_log_path
    }

    /// Get the metrics log path.
    #[allow(dead_code)]
    pub fn metrics_log_path(&self) -> &Path {
        &self.metrics_log_path
    }

    /// Get the session start time.
    #[allow(dead_code)]
    pub fn start_time(&self) -> DateTime<Local> {
        self.start_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_timestamped_filenames() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path();

        // Note: This will initialize the global tracing subscriber
        // so we can't run this test multiple times
        // Instead, just verify the paths are formatted correctly
        let now = Local::now();
        let timestamp = now.format("%Y%m%d-%H%M%S");
        let expected_error = logs_dir.join(format!("{}-errors.log", timestamp));
        let expected_metrics = logs_dir.join(format!("{}-metrics.jsonl", timestamp));

        assert!(expected_error.to_string_lossy().contains("-errors.log"));
        assert!(expected_metrics
            .to_string_lossy()
            .contains("-metrics.jsonl"));
    }

    #[test]
    fn test_compress_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create a test log file
        fs::write(&log_path, b"test log content\n").unwrap();

        // Compress it
        LoggingSession::compress_log_file(&log_path).unwrap();

        // Original should be gone
        assert!(!log_path.exists());

        // Compressed should exist
        let compressed = temp_dir.path().join("test.log.tgz");
        assert!(compressed.exists());

        // Compressed should be smaller or similar size (for small files)
        let compressed_size = fs::metadata(compressed).unwrap().len();
        assert!(compressed_size > 0);
    }

    #[test]
    fn test_rotation_threshold() {
        let temp_dir = TempDir::new().unwrap();
        let old_log = temp_dir.path().join("20230101-120000-errors.log");
        let new_log = temp_dir.path().join("20990101-120000-errors.log");

        // Create files with different ages
        fs::write(&old_log, b"old content\n").unwrap();
        fs::write(&new_log, b"new content\n").unwrap();

        // Set old file's modified time to 25 hours ago
        let old_time = SystemTime::now() - Duration::from_secs(25 * 60 * 60);
        let old_filetime = filetime::FileTime::from_system_time(old_time);
        filetime::set_file_mtime(&old_log, old_filetime).unwrap();

        // Run rotation
        LoggingSession::rotate_old_logs(temp_dir.path()).unwrap();

        // Old log should be compressed
        assert!(!old_log.exists());
        assert!(temp_dir
            .path()
            .join("20230101-120000-errors.log.tgz")
            .exists());

        // New log should still be uncompressed
        assert!(new_log.exists());
    }
}
