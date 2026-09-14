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
#[command(version = "0.1.1")]
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
            let desktop_file = autostart_dir.join("RustCooling.desktop");

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
    info!(" RustCooling v0.1.1 - ID-COOLING FX LCD Controller");
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
        if let Some(w) = win_for_min.upgrade() {
            w.window().set_minimized(true);
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
                w.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
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

    // Smooth, reliable window dragging that preserves Slint pointer capture
    struct DragState {
        is_dragging: bool,
        start_cursor: (i32, i32),
        start_win: (i32, i32),
    }
    let drag_state = Rc::new(RefCell::new(DragState {
        is_dragging: false,
        start_cursor: (0, 0),
        start_win: (0, 0),
    }));

    let drag_for_start = Rc::clone(&drag_state);
    let win_for_drag_start = main_window.as_weak();
    main_window.on_drag_start(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
            let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut pt) };
            if let Some(w) = win_for_drag_start.upgrade() {
                let pos = w.window().position();
                let mut state = drag_for_start.borrow_mut();
                state.is_dragging = true;
                state.start_cursor = (pt.x, pt.y);
                state.start_win = (pos.x, pos.y);
            }
        }
    });

    let drag_for_move = Rc::clone(&drag_state);
    let win_for_drag_move = main_window.as_weak();
    main_window.on_drag_move(move || {
        #[cfg(windows)]
        {
            let state = drag_for_move.borrow();
            if state.is_dragging {
                use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
                let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
                unsafe { GetCursorPos(&mut pt) };
                let dx = pt.x - state.start_cursor.0;
                let dy = pt.y - state.start_cursor.1;
                if let Some(w) = win_for_drag_move.upgrade() {
                    w.window().set_position(slint::PhysicalPosition::new(
                        state.start_win.0 + dx,
                        state.start_win.1 + dy,
                    ));
                }
            }
        }
    });

    let drag_for_end = Rc::clone(&drag_state);
    main_window.on_drag_end(move || {
        drag_for_end.borrow_mut().is_dragging = false;
    });

    // Periodic UI update & tray event polling timer (60ms)
    let handle_for_timer = main_window.as_weak();
    let vis_for_timer = Arc::clone(&is_window_visible);
    let tray_for_timer = Rc::clone(&tray_ref);
    let tray_retry_counter = std::sync::atomic::AtomicUsize::new(0);

    let last_conn = std::cell::Cell::new(false);
    let last_temp = std::cell::Cell::new(-1i32);
    let last_load = std::cell::Cell::new(-1i32);
    let last_label = std::cell::RefCell::new(String::new());
    let last_val = std::cell::RefCell::new(String::new());

    let ui_timer = slint::Timer::default();
    ui_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(60),
        move || {
            // Attempt a single retry for tray initialization if it wasn't ready at startup (after ~1.5s)
            let tick = tray_retry_counter.fetch_add(1, Ordering::Relaxed);
            if tick == 25 && tray_for_timer.borrow().is_none() {
                let vis = vis_for_timer.load(Ordering::SeqCst);
                match SystemTray::new(vis) {
                    Ok(t) => {
                        info!("System tray successfully initialized on retry!");
                        *tray_for_timer.borrow_mut() = Some(t);
                    }
                    Err(e) => {
                        warn!("Tray retry unavailable: {:?}", e);
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
                                w.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
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

            // Sync metrics to UI when visible (throttled/deduplicated)
            if vis_for_timer.load(Ordering::Relaxed) {
                if let Some(w) = handle_for_timer.upgrade() {
                    let is_conn = state.is_connected.load(Ordering::Relaxed);
                    if last_conn.get() != is_conn {
                        last_conn.set(is_conn);
                        w.set_is_connected(is_conn);
                    }

                    if let Ok(m) = state.metrics.lock() {
                        let new_temp = m.temperature.map(|t| t.round() as i32).unwrap_or(0);
                        if last_temp.get() != new_temp {
                            last_temp.set(new_temp);
                            w.set_cpu_temp(new_temp);
                        }

                        let new_load = m.load_percent.map(|l| l.round() as i32).unwrap_or(0);
                        if last_load.get() != new_load {
                            last_load.set(new_load);
                            w.set_cpu_load(new_load);
                        }
                    }

                    if let Ok(lbl) = state.broadcast_label.lock() {
                        if *last_label.borrow() != *lbl {
                            *last_label.borrow_mut() = lbl.clone();
                            w.set_broadcast_label(lbl.as_str().into());
                        }
                    }
                    if let Ok(val) = state.broadcast_value.lock() {
                        if *last_val.borrow() != *val {
                            *last_val.borrow_mut() = val.clone();
                            w.set_broadcast_value(val.as_str().into());
                        }
                    }
                }
            }
        },
    );

    // Center window on screen using Slint's native API before showing
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - 360) / 2;
        let y = (screen_h - 352) / 2;
        main_window
            .window()
            .set_position(slint::PhysicalPosition::new(x, y));
    }

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
        let exe_path = "/usr/local/bin/RustCooling";
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
        assert!(entry.contains("Exec=\"/usr/local/bin/RustCooling\" --minimized"));
        assert!(entry.contains("Type=Application"));
    }

    #[test]
    fn test_main_window_lifecycle_and_interactions() {
        let win = MainWindow::new().expect("Failed to create MainWindow");
        apply_translations(&win);
        assert!(win.show().is_ok());

        // 1. Test Navigation & Callbacks
        let settings_opened = Arc::new(AtomicBool::new(false));
        let settings_opened_clone = Arc::clone(&settings_opened);
        let win_weak1 = win.as_weak();
        win.on_open_settings(move || {
            settings_opened_clone.store(true, Ordering::SeqCst);
            if let Some(w) = win_weak1.upgrade() {
                w.set_show_settings(true);
            }
        });

        let settings_closed = Arc::new(AtomicBool::new(false));
        let settings_closed_clone = Arc::clone(&settings_closed);
        let win_weak2 = win.as_weak();
        win.on_close_settings(move || {
            settings_closed_clone.store(true, Ordering::SeqCst);
            if let Some(w) = win_weak2.upgrade() {
                w.set_show_settings(false);
            }
        });

        let saved = Arc::new(AtomicBool::new(false));
        let saved_clone = Arc::clone(&saved);
        win.on_save_settings(move |_mode, _auto, _int, _lang, _anim, _src, _vid, _pid| {
            saved_clone.store(true, Ordering::SeqCst);
        });

        assert!(!win.get_show_settings(), "Initially should be in monitor view");
        win.invoke_open_settings();
        assert!(settings_opened.load(Ordering::SeqCst), "open_settings callback must fire");
        assert!(win.get_show_settings(), "View must switch to settings");

        win.invoke_save_settings(
            "temp".into(),
            true,
            500,
            "ru".into(),
            "direct".into(),
            "core0".into(),
            "1A86".into(),
            "E317".into(),
        );
        assert!(saved.load(Ordering::SeqCst), "save_settings callback must fire");

        win.invoke_close_settings();
        assert!(settings_closed.load(Ordering::SeqCst), "close_settings callback must fire");
        assert!(!win.get_show_settings(), "View must switch back to monitor view");

        // 2. Test Drag Safety (Drag must never break subsequent button clicks)
        let drag_started = Arc::new(AtomicBool::new(false));
        let drag_moved = Arc::new(AtomicBool::new(false));
        let drag_ended = Arc::new(AtomicBool::new(false));

        let s1 = Arc::clone(&drag_started);
        win.on_drag_start(move || {
            s1.store(true, Ordering::SeqCst);
        });

        let s2 = Arc::clone(&drag_moved);
        win.on_drag_move(move || {
            s2.store(true, Ordering::SeqCst);
        });

        let s3 = Arc::clone(&drag_ended);
        win.on_drag_end(move || {
            s3.store(true, Ordering::SeqCst);
        });

        let button_after_drag = Arc::new(AtomicBool::new(false));
        let bad_clone = Arc::clone(&button_after_drag);
        win.on_open_settings(move || {
            bad_clone.store(true, Ordering::SeqCst);
        });

        win.invoke_drag_start();
        assert!(drag_started.load(Ordering::SeqCst), "drag_start must fire");

        win.invoke_drag_move();
        assert!(drag_moved.load(Ordering::SeqCst), "drag_move must fire");

        win.invoke_drag_end();
        assert!(drag_ended.load(Ordering::SeqCst), "drag_end must fire");

        win.invoke_open_settings();
        assert!(
            button_after_drag.load(Ordering::SeqCst),
            "Button click callback must fire immediately after window drag without freezing!"
        );

        // 3. Test High-Frequency Telemetry Updates & Long-Running Endurance
        let click_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let cc_clone = Arc::clone(&click_count);
        let win_weak3 = win.as_weak();
        win.on_open_settings(move || {
            cc_clone.fetch_add(1, Ordering::SeqCst);
            if let Some(w) = win_weak3.upgrade() {
                let cur = w.get_show_settings();
                w.set_show_settings(!cur);
            }
        });

        for i in 0..600 {
            win.set_cpu_temp(35 + (i % 30) as i32);
            win.set_cpu_load((i % 100) as i32);
            win.set_broadcast_value(format!("{} °C", 35 + (i % 30)).into());
            win.set_is_connected(i % 2 == 0);

            if i % 100 == 50 {
                win.invoke_open_settings();
            }
        }

        let before_final_click = click_count.load(Ordering::SeqCst);
        win.invoke_open_settings();
        let after_final_click = click_count.load(Ordering::SeqCst);

        assert_eq!(
            after_final_click,
            before_final_click + 1,
            "Button click must work reliably after high-frequency continuous telemetry updates"
        );

        win.window().set_minimized(true);
    }
}
