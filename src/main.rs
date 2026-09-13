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
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray::SystemTray;

#[cfg(windows)]
use auto_launch::AutoLaunchBuilder;

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(name = "RustCooling")]
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
        let proc = windows_sys::Win32::System::Threading::GetCurrentProcess();
        EmptyWorkingSet(proc);
    }
}

fn set_autostart(enable: bool) {
    #[cfg(windows)]
    {
        if let Ok(current_exe) = std::env::current_exe() {
            let app_name = "RustCooling";
            let current_exe_str = current_exe.to_string_lossy();
            let auto = AutoLaunchBuilder::new()
                .set_app_name(app_name)
                .set_app_path(&current_exe_str)
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
    #[cfg(target_os = "linux")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let autostart_dir = config_dir.join("autostart");
            let desktop_file = autostart_dir.join("rust-cooling.desktop");

            if enable {
                if let Ok(current_exe) = std::env::current_exe() {
                    if let Err(e) = std::fs::create_dir_all(&autostart_dir) {
                        warn!("Failed to create autostart directory: {e}");
                        return;
                    }
                    let exe_path = current_exe.to_string_lossy();
                    let desktop_entry = format!(
                        "[Desktop Entry]\n\
                         Type=Application\n\
                         Version=1.0\n\
                         Name=RustCooling\n\
                         Comment=ID-COOLING FX LCD Controller\n\
                         Exec=\"{}\" --minimized\n\
                         Terminal=false\n\
                         Categories=Utility;HardwareSettings;\n\
                         StartupNotify=false\n",
                        exe_path
                    );
                    if let Err(e) = std::fs::write(&desktop_file, desktop_entry) {
                        warn!("Failed to write autostart desktop file: {e}");
                    } else {
                        info!("Linux autostart enabled: created {:?}", desktop_file);
                    }
                }
            } else if desktop_file.exists() {
                let _ = std::fs::remove_file(&desktop_file);
                info!("Linux autostart disabled: removed {:?}", desktop_file);
            }
        }
    }
}

fn open_path_or_url(target: &str) {
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let wide: Vec<u16> = std::ffi::OsStr::new(target)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let op: Vec<u16> = std::ffi::OsStr::new("open")
            .encode_wide()
            .chain(Some(0))
            .collect();
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        );
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}

fn apply_translations(w: &MainWindow) {
    let t = I18n::get();
    w.set_tr_app_title(t.app_title.as_str().into());
    w.set_tr_device_name(t.device_name.as_str().into());
    w.set_tr_status_connected(t.status_connected.as_str().into());
    w.set_tr_status_searching(t.status_searching.as_str().into());
    w.set_tr_display_card_title(t.display_card_title.as_str().into());
    w.set_tr_chip_temp(t.chip_temp.as_str().into());
    w.set_tr_chip_load(t.chip_load.as_str().into());
    w.set_tr_btn_settings(t.btn_settings.as_str().into());
    w.set_tr_settings_title(t.settings_title.as_str().into());
    w.set_tr_btn_back(t.btn_back.as_str().into());
    w.set_tr_setting_display_mode(t.setting_display_mode.as_str().into());
    w.set_tr_mode_temp(t.mode_temp.as_str().into());
    w.set_tr_mode_freq(t.mode_freq.as_str().into());
    w.set_tr_mode_load(t.mode_load.as_str().into());
    w.set_tr_setting_interval(t.setting_interval.as_str().into());
    w.set_tr_setting_temp_source(t.setting_temp_source.as_str().into());
    w.set_tr_temp_src_package(t.temp_src_package.as_str().into());
    w.set_tr_temp_src_core0(t.temp_src_core0.as_str().into());
    w.set_tr_temp_src_avg(t.temp_src_avg.as_str().into());
    w.set_tr_temp_src_max(t.temp_src_max.as_str().into());
    w.set_tr_setting_animation(t.setting_animation.as_str().into());
    w.set_tr_anim_smooth(t.anim_smooth.as_str().into());
    w.set_tr_anim_roller(t.anim_roller.as_str().into());
    w.set_tr_anim_direct(t.anim_direct.as_str().into());
    w.set_tr_setting_language(t.setting_language.as_str().into());
    w.set_tr_setting_autostart(t.setting_autostart.as_str().into());
    w.set_tr_autostart_off(t.autostart_off.as_str().into());
    w.set_tr_autostart_on(t.autostart_on.as_str().into());
    w.set_tr_setting_debug(t.setting_debug_title.as_str().into());
    w.set_tr_setting_reset_defaults(t.setting_reset_defaults.as_str().into());
    w.set_tr_open_config_folder(t.setting_open_config.as_str().into());
    w.set_tr_about_title(t.about_title.as_str().into());
    w.set_tr_about_desc(t.about_desc.as_str().into());
}

fn parse_hex_u16(s: &str) -> Option<u16> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u16::from_str_radix(s, 16).ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Ole::OleInitialize;
        let _ = OleInitialize(std::ptr::null_mut());
    }

    let log_dir = AppConfig::config_dir();
    let panic_log_path = log_dir.join("panic.log");
    let debug_log_path = log_dir.join("debug.log");

    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("PANIC OCCURRED: {info}\n");
        let _ = std::fs::write(&panic_log_path, msg);
    }));

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&debug_log_path)
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .target(env_logger::Target::Pipe(Box::new(
                std::io::LineWriter::new(file),
            )))
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

    if config.auto_start {
        set_autostart(true);
    }

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

    let initial_visible = !args.minimized;
    let is_window_visible = Arc::new(AtomicBool::new(initial_visible));

    // Initialize System Tray after Slint/Winit is initialized on the UI thread
    let tray = match SystemTray::new(initial_visible) {
        Ok(t) => {
            info!("System tray successfully initialized!");
            Some(t)
        }
        Err(e) => {
            warn!("Failed to initialize system tray icon: {e}");
            None
        }
    };
    let tray_ref = Rc::new(RefCell::new(tray));

    // Set initial settings and translations
    {
        let cfg = config_ref.lock().unwrap();
        main_window.set_setting_display_mode(cfg.display_mode.as_str().into());
        main_window.set_setting_autostart(cfg.auto_start);
        main_window.set_setting_interval_ms(cfg.update_interval_ms as i32);
        main_window.set_setting_animation(cfg.animation_type.as_str().into());
        main_window.set_setting_language(cfg.language.as_str().into());
        main_window.set_setting_temp_source(cfg.temp_source.as_str().into());
        main_window.set_setting_vid_hex(format!("{:04X}", cfg.custom_vid).into());
        main_window.set_setting_pid_hex(format!("{:04X}", cfg.custom_pid).into());
        main_window.set_device_vid_pid_text(
            format!(
                "USB HID (VID {:04X}, PID {:04X})",
                cfg.custom_vid, cfg.custom_pid
            )
            .into(),
        );
    }
    apply_translations(&main_window);

    // Open URL callback (for GitHub link)
    main_window.on_open_url(|url| {
        let u = url.to_string();
        info!("Opening external URL: {u}");
        open_path_or_url(&u);
    });

    // Open config directory callback
    main_window.on_open_config_folder(|| {
        let path = AppConfig::config_dir();
        info!("Opening config directory: {:?}", path);
        open_path_or_url(&path.to_string_lossy());
    });

    // Reset to defaults callback
    let config_for_reset = Arc::clone(&config_ref);
    let win_for_reset = main_window.as_weak();
    let tray_for_reset = Rc::clone(&tray_ref);
    main_window.on_reset_to_defaults(move || {
        info!("Resetting settings to defaults...");
        let default_cfg = AppConfig::default();
        set_autostart(false);
        let _ = default_cfg.save();
        I18n::set_language(&default_cfg.language);
        if let Ok(mut cfg) = config_for_reset.lock() {
            *cfg = default_cfg.clone();
        }
        if let Some(w) = win_for_reset.upgrade() {
            w.set_setting_display_mode(default_cfg.display_mode.as_str().into());
            w.set_setting_autostart(default_cfg.auto_start);
            w.set_setting_interval_ms(default_cfg.update_interval_ms as i32);
            w.set_setting_animation(default_cfg.animation_type.as_str().into());
            w.set_setting_language(default_cfg.language.as_str().into());
            w.set_setting_temp_source(default_cfg.temp_source.as_str().into());
            w.set_setting_vid_hex(format!("{:04X}", default_cfg.custom_vid).into());
            w.set_setting_pid_hex(format!("{:04X}", default_cfg.custom_pid).into());
            w.set_device_vid_pid_text(
                format!(
                    "USB HID (VID {:04X}, PID {:04X})",
                    default_cfg.custom_vid, default_cfg.custom_pid
                )
                .into(),
            );
            apply_translations(&w);
        }
        if let Some(ref t) = *tray_for_reset.borrow() {
            t.update_labels();
        }
        info!("Reset to defaults complete.");
    });

    // Save settings callback
    let config_for_save = Arc::clone(&config_ref);
    let win_for_save = main_window.as_weak();
    let tray_for_save = Rc::clone(&tray_ref);
    main_window.on_save_settings(move |mode, autostart, interval, lang, anim, temp_src, vid_hex, pid_hex| {
        let lang_str = lang.to_string();
        let anim_str = anim.to_string();
        let temp_src_str = temp_src.to_string();
        I18n::set_language(&lang_str);
        if let Some(w) = win_for_save.upgrade() {
            apply_translations(&w);
        }
        if let Some(ref t) = *tray_for_save.borrow() {
            t.update_labels();
        }
        let parsed_vid = parse_hex_u16(&vid_hex).unwrap_or(0x1A86);
        let parsed_pid = parse_hex_u16(&pid_hex).unwrap_or(0xE317);

        set_autostart(autostart);

        if let Ok(mut cfg) = config_for_save.lock() {
            cfg.display_mode = mode.to_string();
            cfg.auto_start = autostart;
            cfg.update_interval_ms = (interval as u64).clamp(100, 3000);
            cfg.language = lang_str.clone();
            cfg.animation_type = anim_str.clone();
            cfg.temp_source = temp_src_str.clone();
            cfg.custom_vid = parsed_vid;
            cfg.custom_pid = parsed_pid;
            let _ = cfg.save();
            info!("Settings applied: mode={}, autostart={}, interval={}ms, lang={}, anim={}, temp_src={}, vid=0x{:04X}, pid=0x{:04X}",
                mode, autostart, interval, lang_str, anim_str, temp_src_str, parsed_vid, parsed_pid);
        }
        if let Some(w) = win_for_save.upgrade() {
            w.set_device_vid_pid_text(format!("USB HID (VID {:04X}, PID {:04X})", parsed_vid, parsed_pid).into());
        }
    });

    // Open settings view callback
    let win_for_open = main_window.as_weak();
    main_window.on_open_settings(move || {
        info!("UI Event: Open Settings clicked");
        if let Some(w) = win_for_open.upgrade() {
            w.set_show_settings(true);
        }
    });

    // Close settings view callback
    let win_for_close_settings = main_window.as_weak();
    main_window.on_close_settings(move || {
        info!("UI Event: Close Settings clicked");
        if let Some(w) = win_for_close_settings.upgrade() {
            w.set_show_settings(false);
        }
    });

    // Minimize window callback
    let win_for_min = main_window.as_weak();
    main_window.on_minimize_window(move || {
        info!("Minimize requested");
        if let Some(_w) = win_for_min.upgrade() {
            #[cfg(windows)]
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetForegroundWindow, ShowWindow, SW_MINIMIZE,
                };
                let hwnd = GetForegroundWindow();
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_MINIMIZE);
                }
            }
            #[cfg(not(windows))]
            {
                _w.window().set_minimized(true);
            }
            trim_memory();
        }
    });

    // Close window (top-right cross) -> hides window to system tray (or exits if tray unavailable)
    let win_for_close = main_window.as_weak();
    let vis_for_close = Arc::clone(&is_window_visible);
    let tray_for_close = Rc::clone(&tray_ref);
    main_window.on_close_window(move || {
        let has_tray = tray_for_close.borrow().is_some();
        if has_tray {
            info!("Close requested -> hiding window to system tray");
            if let Some(w) = win_for_close.upgrade() {
                let _ = w.hide();
                vis_for_close.store(false, Ordering::SeqCst);
                if let Some(ref t) = *tray_for_close.borrow() {
                    t.set_window_visible(false);
                }
                trim_memory();
            }
        } else {
            info!("Close requested, but no system tray is active -> quitting event loop");
            let _ = slint::quit_event_loop();
        }
    });

    // Native frameless window dragging on Windows
    #[cfg(windows)]
    main_window.on_drag_window(|| {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, SendMessageW, WM_SYSCOMMAND,
        };
        unsafe {
            ReleaseCapture();
            let hwnd = GetForegroundWindow();
            if !hwnd.is_null() {
                SendMessageW(hwnd, WM_SYSCOMMAND, 0xF012, 0);
            }
        }
    });
    #[cfg(not(windows))]
    main_window.on_drag_window(|| {});

    // Periodic UI update & tray event polling timer (~30ms for 33 FPS smooth animations)
    let handle_for_timer = main_window.as_weak();
    let vis_for_timer = Arc::clone(&is_window_visible);
    let first_tick = Arc::new(AtomicBool::new(true));
    let first_tick_clone = Arc::clone(&first_tick);
    let tray_for_timer = Rc::clone(&tray_ref);
    let tray_retry_counter = std::sync::atomic::AtomicUsize::new(0);

    let ui_timer = slint::Timer::default();
    ui_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(30),
        move || {
            if first_tick_clone.swap(false, Ordering::SeqCst) {
                info!("=== FIRST UI TICK: Slint event loop is running smoothly! ===");
                #[cfg(windows)]
                unsafe {
                    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
                    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        EnumWindows, GetSystemMetrics, GetWindowTextW, GetWindowThreadProcessId,
                        SetForegroundWindow, SetWindowPos, SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE,
                        SWP_SHOWWINDOW,
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
                                let y = (screen_h - 352) / 2;
                                SetWindowPos(
                                    hwnd,
                                    std::ptr::null_mut(),
                                    x,
                                    y,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_SHOWWINDOW,
                                );
                                SetForegroundWindow(hwnd);
                                return 0;
                            }
                        }
                        1
                    }
                    EnumWindows(Some(enum_proc), 0);
                }
                trim_memory();
            }

            // Retry tray initialization if it wasn't ready at startup (every ~3 seconds = 100 ticks @ 30ms)
            let tick = tray_retry_counter.fetch_add(1, Ordering::Relaxed);
            if tick % 100 == 99 && tray_for_timer.borrow().is_none() {
                let vis = vis_for_timer.load(Ordering::SeqCst);
                match SystemTray::new(vis) {
                    Ok(t) => {
                        info!("System tray successfully initialized on retry!");
                        *tray_for_timer.borrow_mut() = Some(t);
                    }
                    Err(e) => {
                        warn!("Tray retry failed: {:?}", e);
                    }
                }
            }

            // Poll tray events
            if let Some(ref tray_manager) = *tray_for_timer.borrow() {
                let handle_toggle = handle_for_timer.clone();
                let vis_toggle = Arc::clone(&vis_for_timer);
                let handle_exit = handle_for_timer.clone();
                let tray_inner = Rc::clone(&tray_for_timer);

                tray_manager.poll_events(
                    move || {
                        let currently_visible = vis_toggle.load(Ordering::SeqCst);
                        if currently_visible {
                            info!("Tray action -> hiding window");
                            if let Some(w) = handle_toggle.upgrade() {
                                let _ = w.hide();
                                vis_toggle.store(false, Ordering::SeqCst);
                                if let Some(ref t) = *tray_inner.borrow() {
                                    t.set_window_visible(false);
                                }
                                trim_memory();
                            }
                        } else {
                            info!("Tray action -> restoring window");
                            if let Some(w) = handle_toggle.upgrade() {
                                let _ = w.show();
                                vis_toggle.store(true, Ordering::SeqCst);
                                if let Some(ref t) = *tray_inner.borrow() {
                                    t.set_window_visible(true);
                                }
                                w.window().request_redraw();
                                #[cfg(windows)]
                                unsafe {
                                    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
                                    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
                                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                                        EnumWindows, GetWindowTextW, GetWindowThreadProcessId,
                                        SetForegroundWindow, SetWindowPos, SWP_NOMOVE, SWP_NOSIZE,
                                        SWP_SHOWWINDOW,
                                    };

                                    unsafe extern "system" fn enum_proc(
                                        hwnd: HWND,
                                        _: LPARAM,
                                    ) -> BOOL {
                                        let mut pid = 0;
                                        GetWindowThreadProcessId(hwnd, &mut pid);
                                        if pid == GetCurrentProcessId() {
                                            let mut title = [0u16; 64];
                                            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 64);
                                            let title_str =
                                                String::from_utf16_lossy(&title[..len as usize]);
                                            if title_str.contains("RustCooling") {
                                                SetWindowPos(
                                                    hwnd,
                                                    std::ptr::null_mut(),
                                                    0,
                                                    0,
                                                    0,
                                                    0,
                                                    SWP_NOSIZE | SWP_NOMOVE | SWP_SHOWWINDOW,
                                                );
                                                SetForegroundWindow(hwnd);
                                                return 0;
                                            }
                                        }
                                        1
                                    }
                                    EnumWindows(Some(enum_proc), 0);
                                }
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

    if !args.minimized {
        info!("Step 5: Calling main_window.show()...");
        if let Err(e) = main_window.show() {
            warn!("FAILED main_window.show(): {:?}", e);
            return Err(e.into());
        }
        main_window.window().request_redraw();
        info!("Step 6: main_window.show() returned Ok.");
    } else {
        info!("Step 5: Starting minimized to system tray.");
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
        w.window().set_minimized(true);
    }

    #[test]
    #[cfg(windows)]
    fn test_autostart_builder() {
        let auto = AutoLaunchBuilder::new()
            .set_app_name("RustCoolingTest")
            .set_app_path("C:\\RustCooling.exe")
            .set_args(&["--minimized"])
            .build();
        assert!(auto.is_ok());
    }

    #[test]
    fn test_linux_desktop_entry_format() {
        let exe_path = "/usr/local/bin/rust-cooling";
        let entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Version=1.0\n\
             Name=RustCooling\n\
             Comment=ID-COOLING FX LCD Controller\n\
             Exec=\"{}\" --minimized\n\
             Terminal=false\n\
             Categories=Utility;HardwareSettings;\n\
             StartupNotify=false\n",
            exe_path
        );
        assert!(entry.contains("Exec=\"/usr/local/bin/rust-cooling\" --minimized"));
        assert!(entry.contains("Type=Application"));
    }
}
