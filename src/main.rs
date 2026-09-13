mod config;
mod hid;
mod i18n;
mod protocol;
mod service;
mod telemetry;
mod tray;

use clap::Parser;
use config::AppConfig;
use i18n::I18n;
use log::{info, warn};
use service::MonitorService;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray::SystemTray;

#[cfg(windows)]
use auto_launch::AutoLaunchBuilder;

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(name = "rust-cooling")]
#[command(author = "Qyzom & Contributors")]
#[command(version = "0.1.0")]
#[command(about = "RustCooling - Lightweight LCD Display controller for ID-COOLING FX series coolers", long_about = None)]
struct CliArgs {
    /// Run in headless daemon mode without GUI (for background / systemd)
    #[arg(short, long)]
    daemon: bool,

    /// Start minimized to background / tray
    #[arg(short, long)]
    minimized: bool,

    /// Override update interval in milliseconds
    #[arg(short, long)]
    interval: Option<u64>,
}

pub fn trim_memory() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        let proc = GetCurrentProcess();
        EmptyWorkingSet(proc);
        SetProcessWorkingSetSize(proc, usize::MAX, usize::MAX);
    }
}

#[allow(dead_code)]
fn set_autostart(enable: bool) {

    #[cfg(windows)]
    {
        if let Ok(current_exe) = std::env::current_exe() {
            let app_name = "RustCooling";
            let auto = AutoLaunchBuilder::new()
                .set_app_name(app_name)
                .set_app_path(current_exe.to_str().unwrap_or_default())
                .set_args(&["--minimized"])
                .build();

            if let Ok(auto) = auto {
                if enable {
                    let _ = auto.enable();
                } else {
                    let _ = auto.disable();
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Com::CoInitialize;
        let _ = CoInitialize(std::ptr::null_mut());
    }

    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC OCCURRED: {info}\n");
        let _ = std::fs::write("d:\\Файлы\\antigraivty\\RustCooling\\panic.log", msg);
    }));

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("d:\\Файлы\\antigraivty\\RustCooling\\debug.log")
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .target(env_logger::Target::Pipe(Box::new(file)))
            .init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }
    let args = CliArgs::parse();

    let mut config = AppConfig::load();
    if let Some(interval) = args.interval {
        config.update_interval_ms = interval;
    }
    // Default language is English
    if config.language.is_empty() {
        config.language = "en".to_string();
    }
    I18n::init(&config.language);

    let config_ref = Arc::new(Mutex::new(config));

    info!("==================================================");
    info!(" RustCooling v0.1.0 - ID-COOLING FX LCD Controller");
    info!("==================================================");

    let monitor = MonitorService::new(Arc::clone(&config_ref));
    monitor.start();

    if args.daemon {
        info!("Running in headless daemon mode. Press Enter to exit.");
        trim_memory();
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        info!("Shutting down daemon...");
        monitor.stop();
        return Ok(());
    }

    // Initialize System Tray
    let tray = match SystemTray::new() {
        Ok(t) => Some(t),
        Err(e) => {
            warn!("Failed to initialize system tray icon: {e}");
            None
        }
    };

    // Initialize Slint GUI
    info!("Step 1: Creating MainWindow...");
    let main_window = MainWindow::new()?;
    info!("Step 2: MainWindow created successfully.");
    let state = monitor.get_state();

    // Apply translations to UI
    {
        info!("Step 3: Setting translations...");
        let t = I18n::get();
        main_window.set_str_app_title(t.app_title.into());
        main_window.set_str_app_subtitle(t.app_subtitle.into());
        main_window.set_str_connected(t.connected.into());
        main_window.set_str_disconnected(t.disconnected.into());
        main_window.set_str_display_title(t.display_title.into());
        main_window.set_str_display_desc(t.display_active_metric.into());
        main_window.set_str_chart_load(t.chart_cpu_load.into());
        main_window.set_str_chart_freq(t.chart_cpu_freq.into());
        main_window.set_str_btn_settings(t.btn_settings.into());
        main_window.set_str_btn_minimize(t.btn_minimize.into());
        info!("Step 4: Translations applied.");
    }

    // Callbacks
    main_window.on_open_settings(|| {
        info!("Settings clicked (screen foundation ready)");
    });

    let is_window_visible = Arc::new(AtomicBool::new(!args.minimized));

    // Hide / Minimize to tray
    let handle_for_hide = main_window.as_weak();
    let vis_for_hide = Arc::clone(&is_window_visible);
    let hide_action = move || {
        if let Some(w) = handle_for_hide.upgrade() {
            let _ = w.hide();
            vis_for_hide.store(false, Ordering::SeqCst);
            trim_memory();
        }
    };
    let hide_action_clone = hide_action.clone();
    main_window.on_hide_window(hide_action);
    main_window.on_minimize_window(hide_action_clone);

    // Close window / Quit app
    main_window.on_close_window(|| {
        info!("Close requested. Exiting application...");
        let _ = slint::quit_event_loop();
    });

    // Native frameless window dragging on Windows
    #[cfg(windows)]
    main_window.on_drag_window(|| {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, SendMessageW, HTCAPTION, WM_NCLBUTTONDOWN,
        };
        unsafe {
            ReleaseCapture();
            let hwnd = GetForegroundWindow();
            if !hwnd.is_null() {
                SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
            }
        }
    });
    #[cfg(not(windows))]
    main_window.on_drag_window(|| {});

    // Periodic UI update & tray event polling timer (~150ms)
    let handle_for_timer = main_window.as_weak();
    let vis_for_timer = Arc::clone(&is_window_visible);
    let first_tick = Arc::new(AtomicBool::new(true));
    let first_tick_clone = Arc::clone(&first_tick);
    let ui_timer = slint::Timer::default();
    ui_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(150),
        move || {
            if first_tick_clone.swap(false, Ordering::SeqCst) {
                info!("=== FIRST UI TICK: Slint event loop is running smoothly! ===");
            }

            // Poll tray events
            if let Some(ref tray_manager) = tray {
                let handle_show = handle_for_timer.clone();
                let vis_show = Arc::clone(&vis_for_timer);
                let handle_exit = handle_for_timer.clone();

                tray_manager.poll_events(
                    move || {
                        info!("Tray show/hide toggle clicked");
                        if let Some(w) = handle_show.upgrade() {
                            let currently_visible = vis_show.load(Ordering::SeqCst);
                            if currently_visible {
                                let _ = w.hide();
                                vis_show.store(false, Ordering::SeqCst);
                                trim_memory();
                            } else {
                                let _ = w.show();
                                vis_show.store(true, Ordering::SeqCst);
                            }
                        }
                    },
                    move || {
                        info!("Tray EXIT clicked -> requesting quit_event_loop()");
                        if let Some(_w) = handle_exit.upgrade() {
                            let _ = slint::quit_event_loop();
                        }
                    },
                );
            }

            // Sync metrics to UI when visible
            if vis_for_timer.load(Ordering::Relaxed) {
                if let Some(w) = handle_for_timer.upgrade() {
                    let is_conn = state.is_connected.load(Ordering::Relaxed);
                    w.set_is_connected(is_conn);

                    if let Ok(m) = state.metrics.lock() {
                        if let Some(t) = m.temperature {
                            w.set_cpu_temp(t.round() as i32);
                        } else {
                            w.set_cpu_temp(0);
                        }

                        if let Some(l) = m.load_percent {
                            w.set_cpu_load(l.round() as i32);
                        } else {
                            w.set_cpu_load(0);
                        }

                        if let Some(f) = m.frequency_mhz {
                            w.set_cpu_freq(f.round() as i32);
                        } else {
                            w.set_cpu_freq(0);
                        }
                    }

                    if let Ok(lbl) = state.broadcast_label.lock() {
                        w.set_broadcast_label(lbl.as_str().into());
                    }
                    if let Ok(val) = state.broadcast_value.lock() {
                        w.set_broadcast_value(val.as_str().into());
                    }

                    // Update charts
                    if let Ok(hist) = state.load_history.lock() {
                        let model: Rc<VecModel<f32>> = Rc::new(VecModel::from(hist.clone()));
                        w.set_load_history(ModelRc::from(model));
                    }
                    if let Ok(hist) = state.freq_history.lock() {
                        let model: Rc<VecModel<f32>> = Rc::new(VecModel::from(hist.clone()));
                        w.set_freq_history(ModelRc::from(model));
                    }
                }
            }
        },
    );

    info!("Step 5: Calling main_window.show()...");
    main_window.show()?;
    main_window.window().request_redraw();
    info!("Step 6: main_window.show() returned Ok.");
    info!("Window size: {:?}", main_window.window().size());
    info!("Window is_visible: {:?}", main_window.window().is_visible());
    info!("Window position: {:?}", main_window.window().position());

    if args.minimized {
        info!("Step 7: Minimizing on startup");
        let _ = main_window.hide();
        trim_memory();
    }

    info!("Step 9: Calling main_window.run()...");
    let run_res = main_window.run();
    info!("Step 10: Event loop exited with result: {:?}", run_res);
    monitor.stop();

    Ok(())
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn test_main_window_init() {
        let win = MainWindow::new();
        assert!(win.is_ok(), "MainWindow::new failed: {:?}", win.err());
        let w = win.unwrap();
        let show_res = w.show();
        println!("w.show() result: {:?}", show_res);

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
            use windows_sys::Win32::System::Threading::GetCurrentProcessId;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                EnumWindows, GetWindowThreadProcessId, GetWindowTextW, GetClassNameW, IsWindowVisible,
            };

            unsafe extern "system" fn enum_proc(hwnd: HWND, _: LPARAM) -> BOOL {
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid == GetCurrentProcessId() {
                    let mut title = [0u16; 256];
                    let mut class = [0u16; 256];
                    GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
                    GetClassNameW(hwnd, class.as_mut_ptr(), 256);
                    let title_str = String::from_utf16_lossy(&title);
                    let class_str = String::from_utf16_lossy(&class);
                    let vis = IsWindowVisible(hwnd);
                    println!("FOUND HWND: {:?}, Class: {}, Title: {}, Vis: {}", hwnd, class_str.trim_matches(char::from(0)), title_str.trim_matches(char::from(0)), vis);
                }
                1
            }

            EnumWindows(Some(enum_proc), 0);
        }

        assert!(show_res.is_ok());
    }
}
