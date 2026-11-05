mod config;
mod hid;
mod i18n;
mod protocol;
mod service;
mod telemetry;
mod tui;
pub use service::autostart::{is_autostart_registered, set_autostart, set_autostart_mode};

use clap::Parser;
use config::AppConfig;
use log::{info, warn};
use service::MonitorService;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

static DAEMON_STOP: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
extern "C" fn daemon_sig_handler(_: libc::c_int) {
    DAEMON_STOP.store(true, Ordering::SeqCst);
}

#[derive(Parser, Debug)]
#[command(name = "rustcooling")]
#[command(author = "Qyzom & Contributors")]
#[command(version)]
#[command(
    about = "RustCooling CLI - Ultra-lightweight terminal interface and daemon for ID-COOLING FX series coolers",
    long_about = None
)]
struct CliArgs {
    /// Run in interactive TUI mode (default when run without arguments)
    #[arg(short, long)]
    tui: bool,

    /// Run in headless daemon mode without GUI (for background / systemd)
    #[arg(short, long)]
    daemon: bool,

    /// Override update interval in milliseconds
    #[arg(short, long)]
    interval: Option<u64>,

    /// Print current telemetry status and exit
    #[arg(short, long)]
    status: bool,

    /// Stop running background daemon and turn off LCD screen
    #[arg(long)]
    stop: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = std::env::args().collect();
    let is_daemon = raw_args.iter().any(|a| a == "-d" || a == "--daemon");
    let is_status = raw_args.iter().any(|a| a == "-s" || a == "--status");

    // Suppress stderr logger during interactive TUI mode to prevent terminal artifacts
    if !is_daemon && !is_status {
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off"))
            .is_test(false)
            .try_init();
    } else {
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .is_test(false)
            .try_init();
    }

    let args = CliArgs::parse();

    let mut cfg = AppConfig::load();
    if let Some(interval) = args.interval {
        cfg.update_interval_ms = interval.clamp(100, 3000);
    }
    i18n::I18n::set_language(&cfg.language);
    let config_ref = Arc::new(std::sync::Mutex::new(cfg));

    if args.stop {
        info!("Stopping background daemon and shutting down display...");
        let stopped = service::lifecycle::stop_daemon();
        let monitor = MonitorService::new(Arc::clone(&config_ref));
        monitor.power_off_on_stop();
        monitor.stop();
        if stopped {
            println!("Daemon stopped and display powered off.");
        } else {
            println!("Display powered off (no active daemon was detected).");
        }
        return Ok(());
    }

    if args.status {
        let monitor = MonitorService::new(Arc::clone(&config_ref));
        monitor.start();
        std::thread::sleep(Duration::from_millis(400));
        let state = monitor.get_state();
        let is_conn = state.is_connected.load(Ordering::Relaxed);
        let (temp, load) = if let Ok(m) = state.metrics.lock() {
            (
                m.temperature.map(|t| t.round() as i32).unwrap_or(0),
                m.load_percent.map(|l| l.round() as i32).unwrap_or(0),
            )
        } else {
            (0, 0)
        };
        let b_val = state
            .broadcast_value
            .lock()
            .map(|v| v.clone())
            .unwrap_or_else(|_| "0".to_string());
        println!("RustCooling Telemetry Status:");
        println!("  Connected:      {}", is_conn);
        println!("  CPU Temp:       {} °C", temp);
        println!("  CPU Load:       {} %", load);
        println!("  Display Output: {}", b_val);
        monitor.leave_screen_on();
        monitor.stop();
        return Ok(());
    }

    if args.daemon {
        if let Some(pid) = service::lifecycle::get_daemon_pid() {
            if pid != std::process::id() {
                info!("Another daemon instance is already active (PID {}). Exiting.", pid);
                return Ok(());
            }
        }
        service::lifecycle::write_daemon_pid(std::process::id());

        let monitor = Arc::new(MonitorService::new(Arc::clone(&config_ref)));
        monitor.start();

        info!(
            "RustCooling CLI running in headless daemon mode (~2MB RAM, PID {}).",
            std::process::id()
        );

        #[cfg(target_os = "linux")]
        unsafe {
            libc::signal(libc::SIGHUP, libc::SIG_IGN);
            libc::signal(libc::SIGINT, daemon_sig_handler as *const () as usize);
            libc::signal(libc::SIGTERM, daemon_sig_handler as *const () as usize);
        }

        while !DAEMON_STOP.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(500));
        }

        info!("Shutting down daemon...");
        service::lifecycle::remove_daemon_pid();
        monitor.stop();
        return Ok(());
    }

    // Default action for CLI version is directly opening TUI mode
    info!("Launching interactive TUI mode...");
    service::lifecycle::stop_daemon();
    let monitor = Arc::new(MonitorService::new(Arc::clone(&config_ref)));
    monitor.leave_screen_on();
    monitor.start();

    #[cfg(target_os = "linux")]
    unsafe {
        extern "C" fn tui_sighup_handler(_: libc::c_int) {
            let _ = crate::service::lifecycle::spawn_daemon();
            unsafe {
                libc::_exit(0);
            }
        }
        libc::signal(libc::SIGHUP, tui_sighup_handler as *const () as usize);
        libc::signal(libc::SIGTERM, tui_sighup_handler as *const () as usize);
    }

    let mut app = tui::TuiApp::new(Arc::clone(&config_ref), Arc::clone(&monitor));
    let keep_daemon = app.run().unwrap_or(true);

    if keep_daemon {
        monitor.leave_screen_on();
        monitor.stop();
        if !service::lifecycle::spawn_daemon() {
            warn!("Failed to spawn daemon process.");
        }
    } else {
        monitor.power_off_on_stop();
        monitor.stop();
    }
    Ok(())
}
