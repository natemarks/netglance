use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

mod config;
mod constants;
mod logging;
mod metrics;
mod probe;
mod ui;

use config::{ensure_config_dir, logs_dir, ConfigFile, ProbeConfig};
use logging::LoggingSession;
use metrics::{MetricsStore, MetricsUpdater};
use probe::ProbeScheduler;
use ui::{restore_terminal, setup_terminal, App, Event, EventHandler};

/// TCP Connection Monitor TUI
#[derive(Parser, Debug)]
#[command(name = "netglance")]
#[command(about = "Monitor TCP connections with latency tracking", long_about = None)]
#[command(version = concat!(env!("BUILD_VERSION"), " (", env!("GIT_HASH"), ")"))]
struct Args {
    /// Path to configuration file (default: $HOME/.netglance/settings.json)
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Generate default configuration file and exit
    #[arg(long)]
    init_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Handle --init-config command
    if args.init_config {
        return init_config_file();
    }

    // Load configuration
    let (config_file, hosts) = load_configuration(&args)?;

    // Set up logging session with timestamped files
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .unwrap_or_else(|| config_file.logging.level.clone());
    let logging_session = LoggingSession::new(&logs_dir()?, &log_level)?;

    info!("Starting netglance TCP connection monitor");
    info!("Configuration loaded successfully");

    // Validate hosts
    if hosts.is_empty() {
        bail!("No hosts to monitor. Use --init-config to create configuration, or specify hosts on command line.");
    }

    info!("Monitoring {} host(s)", hosts.len());
    for (name, address) in &hosts {
        info!("  {} -> {}", name, address);
    }

    // Create probe configuration
    let probe_config = config_file.to_probe_config();

    info!(
        "Configuration: interval={}s, timeout={}s, window={}m",
        probe_config.interval.as_secs(),
        probe_config.timeout.as_secs(),
        probe_config.window_duration.as_secs() / 60
    );

    // Validate configuration
    let warnings = probe_config.validate();
    for warning in &warnings {
        warn!("{}", warning);
    }

    // Run the application
    run_app(hosts, probe_config, logging_session).await
}

/// Initialize configuration file.
fn init_config_file() -> Result<()> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join("settings.json");

    if config_path.exists() {
        println!(
            "Configuration file already exists at: {}",
            config_path.display()
        );
        println!("Edit the file to customize your monitoring setup.");
        println!("Or use --config to specify a different location.");
        return Ok(());
    }

    // Create default configuration
    let default_config = ConfigFile::default_config();
    default_config.save(&config_path)?;

    println!("✓ Created configuration file: {}", config_path.display());
    println!("✓ Created log directory: {}/logs", config_dir.display());
    println!();
    println!("Edit the file to customize your monitoring setup:");
    println!("  vim {}", config_path.display());
    println!();
    println!("Then start monitoring:");
    println!("  netglance");

    Ok(())
}

/// Load configuration from file.
fn load_configuration(args: &Args) -> Result<(ConfigFile, Vec<(String, String)>)> {
    let config_file = if let Some(ref path) = args.config {
        // User specified config file
        info!("Loading configuration from: {}", path.display());
        ConfigFile::load(path).context("Failed to load specified config file")?
    } else {
        // Try to find default config file
        let default_path = config::default_config_path()?;
        if default_path.exists() {
            info!("Loading configuration from: {}", default_path.display());
            ConfigFile::load(&default_path).with_context(|| {
                format!(
                    "Failed to load default config file: {}",
                    default_path.display()
                )
            })?
        } else {
            bail!(
                "No configuration file found.\n\
                 \n\
                 Use 'netglance --init-config' to create one at:\n\
                 {}",
                default_path.display()
            );
        }
    };

    let hosts = config_file.get_hosts();
    Ok((config_file, hosts))
}

/// Run the complete application with all components wired together.
async fn run_app(
    hosts: Vec<(String, String)>,
    config: ProbeConfig,
    logging_session: LoggingSession,
) -> Result<()> {
    // Add all hosts to the metrics store immediately
    // They will show as "Waiting" until the first probe completes
    info!("Configuring {} host(s) for monitoring...", hosts.len());

    let all_hosts_for_store: Vec<(String, String, String)> = hosts
        .iter()
        .map(|(hostname, address)| {
            info!("  {} -> {}", hostname, address);
            // Use address as placeholder for resolved_address
            // The probe scheduler will handle DNS resolution
            (hostname.clone(), address.clone(), address.clone())
        })
        .collect();

    // Create metrics store with ALL hosts
    // Hosts will show as "⧗ WAITING" until first probe result
    let metrics_store = Arc::new(RwLock::new(MetricsStore::new(
        all_hosts_for_store.clone(),
        config.clone(),
    )));

    // Create channel for probe results
    let (probe_tx, probe_rx) = mpsc::unbounded_channel();

    // Create cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

    // Get all addresses for probing
    // DNS resolution and connection testing will be handled by the probe scheduler
    let addresses: Vec<String> = hosts.iter().map(|(_, address)| address.clone()).collect();

    // Create probe scheduler
    let probe_scheduler = ProbeScheduler::new(
        addresses.clone(),
        config.interval,
        config.timeout,
        probe_tx,
        cancel_token.clone(),
    );

    // Create metrics updater
    let metrics_updater = MetricsUpdater::new(
        metrics_store.clone(),
        probe_rx,
        Duration::from_secs(1), // Update metrics every second
        cancel_token.clone(),
    );

    // Spawn probe scheduler task
    let scheduler_handle = tokio::spawn(async move {
        info!("Starting probe scheduler");
        probe_scheduler.run().await;
        info!("Probe scheduler stopped");
    });

    // Spawn metrics updater task
    let updater_handle = tokio::spawn(async move {
        info!("Starting metrics updater");
        metrics_updater.run().await;
        info!("Metrics updater stopped");
    });

    // Set up terminal
    let mut terminal = setup_terminal().context("Failed to setup terminal")?;

    // Create application state
    let mut app = App::new(metrics_store.clone());

    // Create event handler (10 FPS refresh rate)
    let (event_handler, mut event_receiver) = EventHandler::new(Duration::from_millis(100));
    event_handler.run().await;

    info!("Starting TUI main loop");

    // Run the main event loop with metrics logging
    let ui_result = run_event_loop(
        &mut terminal,
        &mut app,
        &mut event_receiver,
        metrics_store.clone(),
        &logging_session,
    )
    .await;

    info!("TUI main loop stopped");

    // Signal shutdown to background tasks
    cancel_token.cancel();

    // Wait for background tasks to complete (with timeout)
    let shutdown_timeout = Duration::from_secs(2);
    let scheduler_result = tokio::time::timeout(shutdown_timeout, scheduler_handle).await;
    let updater_result = tokio::time::timeout(shutdown_timeout, updater_handle).await;

    if scheduler_result.is_err() {
        error!("Probe scheduler did not shut down within timeout");
    }
    if updater_result.is_err() {
        error!("Metrics updater did not shut down within timeout");
    }

    // Restore terminal
    restore_terminal().context("Failed to restore terminal")?;

    info!("Application shut down complete");

    ui_result
}

/// Run the main application event loop.
async fn run_event_loop(
    terminal: &mut ratatui::Terminal<ui::terminal::TerminalBackend>,
    app: &mut App,
    event_receiver: &mut mpsc::UnboundedReceiver<Event>,
    metrics_store: Arc<RwLock<MetricsStore>>,
    logging_session: &LoggingSession,
) -> Result<()> {
    let mut metrics_log_interval = tokio::time::interval(constants::metrics_log_interval());
    metrics_log_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = metrics_log_interval.tick() => {
                // Log metrics snapshot every 10 seconds
                let store = metrics_store.read().await;
                let snapshot = store.snapshot(Instant::now());
                drop(store);

                if let Err(e) = logging_session.log_metrics(&snapshot) {
                    error!("Failed to log metrics: {}", e);
                }
            }

            event_opt = event_receiver.recv() => {
                // Draw UI
                terminal.draw(|frame| {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async { ui::render::render(frame, app).await })
                    });
                })?;

                // Handle events
                if let Some(event) = event_opt {
                    match event {
                        Event::Key(key) => {
                            app.handle_key_event(key);
                        }
                        Event::Tick => {
                            // UI refresh happens automatically
                        }
                        Event::Resize(_, _) => {
                            // Terminal will redraw automatically
                        }
                    }
                }

                // Exit if requested
                if app.should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}
