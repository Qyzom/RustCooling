#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
use slint::ComponentHandle;
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

fn apply_translations(w: &MainWindow) {
    let t = I18n::get();
    w.set_tr_app_title(t.app_title.as_str().into());
    w.set_tr_app_badge(t.app_badge.as_str().into());
    w.set_tr_device_name(t.device_name.as_str().into());
    w.set_tr_device_desc_connected(t.device_desc_connected.as_str().into());
    w.set_tr_device_desc_searching(t.device_desc_searching.as_str().into());
    w.set_tr_status_connected(t.status_connected.as_str().into());
    w.set_tr_status_searching(t.status_searching.as_str().into());
    w.set_tr_display_card_title(t.display_card_title.as_str().into());
    w.set_tr_chip_temp(t.chip_temp.as_str().into());
    w.set_tr_chip_freq(t.chip_freq.as_str().into());
    w.set_tr_chip_load(t.chip_load.as_str().into());
    w.set_tr_btn_settings(t.btn_settings.as_str().into());
    w.set_tr_settings_title(t.settings_title.as_str().into());
    w.set_tr_btn_back(t.btn_back.as_str().into());
    w.set_tr_setting_display_mode(t.setting_display_mode.as_str().into());
    w.set_tr_mode_temp(t.mode_temp.as_str().into());
    w.set_tr_mode_freq(t.mode_freq.as_str().into());
    w.set_tr_mode_load(t.mode_load.as_str().into());
    w.set_tr_mode_carousel(t.mode_carousel.as_str().into());
    w.set_tr_setting_freq_format(t.setting_freq_format.as_str().into());
    w.set_tr_freq_ghz(t.freq_ghz.as_str().into());
    w.set_tr_freq_mhz(t.freq_mhz.as_str().into());
    w.set_tr_setting_language(t.setting_language.as_str().into());
    w.set_tr_btn_save_return(t.btn_save_return.as_str().into());
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
            .target(env_logger::Target::Pipe(Box::new(std::io::LineWriter::new(file))))
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
    let tray_ref = Arc::new(Mutex::new(tray));

    // Initialize Slint GUI
    let main_window = match MainWindow::new() {
        Ok(w) => {
            info!("Step 2: MainWindow created successfully.");
            w
        }
        Err(e) => {
            warn!("FAILED to create MainWindow: {:?}", e);
            return Err(e.into());
        }
    };
    let state = monitor.get_state();

    // Set initial settings and translations
    {
        let cfg = config_ref.lock().unwrap();
        main_window.set_setting_display_mode(cfg.display_mode.as_str().into());
        main_window.set_setting_freq_ghz(cfg.freq_in_ghz);
        main_window.set_setting_interval_ms(cfg.update_interval_ms as i32);
        main_window.set_setting_language(cfg.language.as_str().into());
    }
    apply_translations(&main_window);

    // Callbacks
    let config_for_save = Arc::clone(&config_ref);
    let win_for_save = main_window.as_weak();
    let tray_for_save = Arc::clone(&tray_ref);
    main_window.on_save_settings(move |mode, freq_ghz, interval, lang| {
        let lang_str = lang.to_string();
        I18n::set_language(&lang_str);
        if let Some(w) = win_for_save.upgrade() {
            apply_translations(&w);
        }
        if let Ok(guard) = tray_for_save.lock() {
            if let Some(ref t) = *guard {
                t.update_labels();
            }
        }
        if let Ok(mut cfg) = config_for_save.lock() {
            cfg.display_mode = mode.to_string();
            cfg.freq_in_ghz = freq_ghz;
            cfg.update_interval_ms = (interval as u64).max(300);
            cfg.language = lang_str.clone();
            let _ = cfg.save();
            info!("Settings applied: mode={}, freq_ghz={}, interval={}ms, lang={}", mode, freq_ghz, interval, lang_str);
        }
    });

    let is_window_visible = Arc::new(AtomicBool::new(!args.minimized));

    // Minimize window callback
    let win_for_min = main_window.as_weak();
    main_window.on_minimize_window(move || {
        info!("Minimize requested");
        if let Some(_w) = win_for_min.upgrade() {
            #[cfg(windows)]
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, ShowWindow, SW_MINIMIZE};
                let hwnd = GetForegroundWindow();
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_MINIMIZE);
                }
            }
            trim_memory();
        }
    });

    // Close window (top-right cross) -> hides window to system tray
    let win_for_close = main_window.as_weak();
    let vis_for_close = Arc::clone(&is_window_visible);
    main_window.on_close_window(move || {
        info!("Close requested -> hiding window to tray");
        if let Some(w) = win_for_close.upgrade() {
            let _ = w.hide();
            vis_for_close.store(false, Ordering::SeqCst);
            trim_memory();
        }
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
    let tray_for_timer = Arc::clone(&tray_ref);

    let ui_timer = slint::Timer::default();
    ui_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(150),
        move || {
            if first_tick_clone.swap(false, Ordering::SeqCst) {
                info!("=== FIRST UI TICK: Slint event loop is running smoothly! ===");
                trim_memory();
            }

            // Retry tray initialization if it wasn't ready at startup
            if let Ok(mut guard) = tray_for_timer.lock() {
                if guard.is_none() {
                    if let Ok(t) = SystemTray::new() {
                        info!("System tray successfully initialized on retry!");
                        *guard = Some(t);
                    }
                }
            }

            // Poll tray events
            if let Ok(guard) = tray_for_timer.lock() {
                if let Some(ref tray_manager) = *guard {
                    let handle_show = handle_for_timer.clone();
                    let vis_show = Arc::clone(&vis_for_timer);
                    let handle_exit = handle_for_timer.clone();

                    tray_manager.poll_events(
                        move || {
                            info!("Tray restore requested -> showing window");
                            if let Some(w) = handle_show.upgrade() {
                                let _ = w.show();
                                vis_show.store(true, Ordering::SeqCst);
                                w.window().request_redraw();
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
                }
            }
        },
    );

    info!("Step 5: Calling main_window.show()...");
    if let Err(e) = main_window.show() {
        warn!("FAILED main_window.show(): {:?}", e);
        return Err(e.into());
    }
    main_window.window().request_redraw();
    info!("Step 6: main_window.show() returned Ok.");

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetSystemMetrics, GetWindowTextW, GetWindowThreadProcessId, SetWindowPos,
            SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE, SWP_NOZORDER,
        };

        unsafe extern "system" fn enum_proc(hwnd: HWND, _: LPARAM) -> BOOL {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == GetCurrentProcessId() {
                let mut title = [0u16; 64];
                let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 64);
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                if title_str.contains("RustCooling") {
                    let screen_w = GetSystemMetrics(SM_CXSCREEN);
                    let screen_h = GetSystemMetrics(SM_CYSCREEN);
                    let x = (screen_w - 360) / 2;
                    let y = (screen_h - 380) / 2;
                    SetWindowPos(hwnd, std::ptr::null_mut(), x, y, 0, 0, SWP_NOZORDER | SWP_NOSIZE);
                    return 0;
                }
            }
            1
        }
        EnumWindows(Some(enum_proc), 0);
    }

    info!("Window size: {:?}", main_window.window().size());
    info!("Window is_visible: {:?}", main_window.window().is_visible());
    info!("Window position: {:?}", main_window.window().position());

    if args.minimized {
        info!("Step 7: Minimizing on startup");
        let _ = main_window.hide();
        trim_memory();
    }

    info!("Step 9: Calling slint::run_event_loop_until_quit()...");
    let run_res = slint::run_event_loop_until_quit();
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
        apply_translations(&w);
        let show_res = w.show();
        assert!(show_res.is_ok());
    }
}
