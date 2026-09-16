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
#[command(version = "0.1.3")]
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

    /// Install Ring 0 driver service (internal helper called with elevated privileges)
    #[arg(long)]
    install_driver: bool,
}

pub fn show_and_focus_window(w: &MainWindow) {
    w.window().set_minimized(false);
    let _ = w.show();
    w.window().request_redraw();
}

pub fn hide_window_to_tray(w: &MainWindow) {
    w.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
    let _ = w.hide();
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

pub fn is_autostart_registered() -> bool {
    #[cfg(windows)]
    {
        if let Ok(current_exe) = std::env::current_exe() {
            let app_name = "RustCooling";
            let current_exe_str = current_exe.to_string_lossy();
            if let Ok(auto) = AutoLaunchBuilder::new()
                .set_app_name(app_name)
                .set_app_path(&current_exe_str)
                .set_args(&["--minimized"])
                .build()
            {
                return auto.is_enabled().unwrap_or(false);
            }
        }
        false
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let desktop_file = config_dir.join("autostart").join("RustCooling.desktop");
            return desktop_file.exists();
        }
        false
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
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
    w.set_tr_mode_load(t.mode_load.as_str().into());
    w.set_tr_setting_interval(t.setting_interval.as_str().into());
    w.set_tr_setting_temp_source(t.setting_temp_source.as_str().into());
    w.set_tr_setting_temp_smoothing(t.setting_temp_smoothing.as_str().into());
    w.set_tr_smoothing_off(t.smoothing_off.as_str().into());
    w.set_tr_temp_src_package(t.temp_src_package.as_str().into());
    w.set_tr_temp_src_core0(t.temp_src_core0.as_str().into());
    w.set_tr_temp_src_avg(t.temp_src_avg.as_str().into());
    w.set_tr_temp_src_max(t.temp_src_max.as_str().into());
    w.set_tr_setting_animation(t.setting_animation.as_str().into());
    w.set_tr_anim_disabled(t.anim_disabled.as_str().into());
    w.set_tr_anim_enabled(t.anim_enabled.as_str().into());
    w.set_tr_setting_language(t.setting_language.as_str().into());
    w.set_tr_setting_autostart(t.setting_autostart.as_str().into());
    w.set_tr_autostart_off(t.autostart_off.as_str().into());
    w.set_tr_autostart_on(t.autostart_on.as_str().into());
    w.set_tr_setting_debug(t.setting_debug_title.as_str().into());
    w.set_tr_setting_reset_defaults(t.setting_reset_defaults.as_str().into());
    w.set_tr_open_config_folder(t.setting_open_config.as_str().into());
    w.set_tr_about_title(t.about_title.as_str().into());
    w.set_tr_about_desc(t.about_desc.as_str().into());
    w.set_tr_activation_title(t.activation_title.as_str().into());
    w.set_tr_activation_subtitle(t.activation_subtitle.as_str().into());
    w.set_tr_activation_driver_title(t.activation_driver_title.as_str().into());
    w.set_tr_activation_driver_desc(t.activation_driver_desc.as_str().into());
    w.set_tr_activation_driver_btn(t.activation_driver_btn.as_str().into());
    w.set_tr_activation_driver_installed(t.activation_driver_installed.as_str().into());
    w.set_tr_activation_continue_btn(t.activation_continue_btn.as_str().into());
    w.set_tr_setting_driver_title(t.setting_driver_title.as_str().into());
    w.set_tr_driver_status_active(t.driver_status_active.as_str().into());
    w.set_tr_driver_status_missing(t.driver_status_missing.as_str().into());
    w.set_tr_driver_btn_install(t.driver_btn_install.as_str().into());
}

fn sync_ui_from_config(w: &MainWindow, cfg: &AppConfig, autostart: bool, driver_installed: bool) {
    w.set_setting_display_mode(cfg.display_mode.as_str().into());
    w.set_setting_autostart(autostart);
    w.set_setting_interval_ms(cfg.update_interval_ms as i32);
    w.set_setting_animation_enabled(cfg.animation_enabled);
    w.set_setting_language(cfg.language.as_str().into());
    w.set_setting_temp_source(cfg.temp_source.as_str().into());
    w.set_setting_temp_smoothing(cfg.temp_smoothing as i32);
    w.set_setting_vid_hex(format!("{:04X}", cfg.custom_vid).into());
    w.set_setting_pid_hex(format!("{:04X}", cfg.custom_pid).into());
    w.set_device_vid_pid_text(
        format!(
            "USB HID (VID {:04X}, PID {:04X})",
            cfg.custom_vid, cfg.custom_pid
        )
        .into(),
    );
    w.set_is_driver_installed(driver_installed);
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
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, SetPriorityClass, ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        };
        if SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS) == 0 {
            SetPriorityClass(GetCurrentProcess(), ABOVE_NORMAL_PRIORITY_CLASS);
        }
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = CliArgs::parse();

    #[cfg(windows)]
    if args.install_driver {
        info!("Running with --install-driver. Setting up WinRing0 service...");
        match telemetry::driver::install_service() {
            Ok(_) => {
                info!("Driver service successfully installed!");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error installing driver: {}", e);
                std::process::exit(1);
            }
        }
    }

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
    info!(" RustCooling v0.1.3 - ID-COOLING FX LCD Controller");
    info!("==================================================");

    let monitor = MonitorService::new(Arc::clone(&config_ref));
    monitor.start();

    if args.daemon {
        info!("Running in headless daemon mode. Press Enter to exit.");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        info!("Shutting down daemon...");
        monitor.stop();
        #[cfg(windows)]
        {
            let _ = crate::telemetry::driver::stop_service();
        }
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

    // Instant event handler for tray icon and context menu
    let win_handle_for_tray = main_window.as_weak();
    let vis_for_tray = Arc::clone(&is_window_visible);

    let on_tray_action = move |action: tray::TrayAction| {
        let win_weak = win_handle_for_tray.clone();
        let vis = Arc::clone(&vis_for_tray);

        let _ = slint::invoke_from_event_loop(move || {
            match action {
                tray::TrayAction::Show => {
                    info!("Tray event: Show requested");
                    if let Some(w) = win_weak.upgrade() {
                        show_and_focus_window(&w);
                        vis.store(true, Ordering::SeqCst);
                    }
                }
                tray::TrayAction::Toggle => {
                    info!("Tray event: Toggle requested");
                    if let Some(w) = win_weak.upgrade() {
                        let is_showing = vis.load(Ordering::SeqCst);
                        if is_showing {
                            hide_window_to_tray(&w);
                            vis.store(false, Ordering::SeqCst);
                        } else {
                            show_and_focus_window(&w);
                            vis.store(true, Ordering::SeqCst);
                        }
                    }
                }
                tray::TrayAction::Exit => {
                    info!("Tray event: Exit requested");
                    let _ = slint::quit_event_loop();
                }
            }
        });
    };

    let on_tray_action_retry = on_tray_action.clone();

    // Initialize System Tray after Slint/Winit is initialized on the UI thread
    let tray = match SystemTray::new(on_tray_action) {
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
        let actual_autostart = is_autostart_registered();
        #[cfg(windows)]
        let driver_installed = telemetry::driver::is_driver_accessible();
        #[cfg(not(windows))]
        let driver_installed = true;

        sync_ui_from_config(&main_window, &cfg, actual_autostart, driver_installed);

        #[cfg(windows)]
        let is_activation = !cfg.first_run_completed && !driver_installed;
        #[cfg(not(windows))]
        let is_activation = false;

        main_window.set_is_activation_mode(is_activation);
    }
    apply_translations(&main_window);

    // Request driver installation callback (UAC elevation)
    let win_for_driver = main_window.as_weak();
    main_window.on_request_install_driver(move || {
        info!("UI Event: Request Install Driver clicked");
        #[cfg(windows)]
        {
            if let Err(e) = telemetry::driver::request_elevation_install() {
                warn!("Failed to request elevation: {}", e);
            } else {
                let win_clone = win_for_driver.clone();
                slint::Timer::single_shot(Duration::from_millis(1500), move || {
                    let accessible = telemetry::driver::is_driver_accessible();
                    info!("Driver accessibility check after elevation: {}", accessible);
                    if let Some(w) = win_clone.upgrade() {
                        w.set_is_driver_installed(accessible);
                    }
                });
            }
        }
    });

    // Finish activation callback
    let config_for_finish = Arc::clone(&config_ref);
    let win_for_finish = main_window.as_weak();
    main_window.on_finish_activation(move || {
        info!("UI Event: Finish Activation clicked");
        if let Ok(mut cfg) = config_for_finish.lock() {
            cfg.first_run_completed = true;
            let _ = cfg.save();
        }
        if let Some(w) = win_for_finish.upgrade() {
            w.set_is_activation_mode(false);
        }
    });

    // Change language callback in activation view
    let config_for_lang = Arc::clone(&config_ref);
    let win_for_lang = main_window.as_weak();
    let tray_for_lang = Rc::clone(&tray_ref);
    main_window.on_change_language(move |lang| {
        let lang_str = lang.to_string();
        info!("UI Event: Change Language to {}", lang_str);
        I18n::set_language(&lang_str);
        if let Ok(mut cfg) = config_for_lang.lock() {
            cfg.language = lang_str.clone();
            let _ = cfg.save();
        }
        if let Some(w) = win_for_lang.upgrade() {
            w.set_setting_language(lang_str.as_str().into());
            apply_translations(&w);
        }
        if let Some(ref t) = *tray_for_lang.borrow() {
            t.update_labels();
        }
    });

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
        #[cfg(windows)]
        {
            info!("Re-installing WinRing0 driver on reset...");
            let _ = telemetry::driver::install_service();
        }
        #[cfg(windows)]
        let driver_installed = telemetry::driver::is_driver_accessible();
        #[cfg(not(windows))]
        let driver_installed = true;

        if let Some(w) = win_for_reset.upgrade() {
            sync_ui_from_config(&w, &default_cfg, false, driver_installed);
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
    main_window.on_save_settings(move |mode, autostart, interval, lang, anim_enabled, temp_src, vid_hex, pid_hex, temp_smoothing| {
        let lang_str = lang.to_string();
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
        let actual_autostart = is_autostart_registered();

        if let Ok(mut cfg) = config_for_save.lock() {
            cfg.display_mode = mode.to_string();
            cfg.auto_start = actual_autostart;
            cfg.update_interval_ms = (interval as u64).clamp(100, 3000);
            cfg.language = lang_str.clone();
            cfg.animation_enabled = anim_enabled;
            cfg.temp_source = temp_src_str.clone();
            cfg.temp_smoothing = (temp_smoothing as u32).min(5);
            cfg.custom_vid = parsed_vid;
            cfg.custom_pid = parsed_pid;
            let _ = cfg.save();
            info!("Settings applied: mode={}, autostart={}, interval={}ms, lang={}, anim_enabled={}, temp_src={}, smoothing={}C, vid=0x{:04X}, pid=0x{:04X}",
                mode, actual_autostart, interval, lang_str, anim_enabled, temp_src_str, cfg.temp_smoothing, parsed_vid, parsed_pid);
        }
        if let Some(w) = win_for_save.upgrade() {
            w.set_setting_autostart(actual_autostart);
            w.set_device_vid_pid_text(format!("USB HID (VID {:04X}, PID {:04X})", parsed_vid, parsed_pid).into());
        }
    });

    // Open settings view callback
    let win_for_open = main_window.as_weak();
    main_window.on_open_settings(move || {
        info!("UI Event: Open Settings clicked");
        if let Some(w) = win_for_open.upgrade() {
            let actual_autostart = is_autostart_registered();
            w.set_setting_autostart(actual_autostart);
            #[cfg(windows)]
            w.set_is_driver_installed(telemetry::driver::is_driver_accessible());
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

    // Close window (top-right cross) -> hides window to system tray (or exits if tray unavailable)
    let win_for_close = main_window.as_weak();
    let vis_for_close = Arc::clone(&is_window_visible);
    let tray_for_close = Rc::clone(&tray_ref);
    main_window.on_close_window(move || {
        let has_tray = tray_for_close.borrow().is_some();
        if has_tray {
            info!("Close requested -> hiding window to system tray");
            if let Some(w) = win_for_close.upgrade() {
                hide_window_to_tray(&w);
                vis_for_close.store(false, Ordering::SeqCst);
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

    // Periodic UI update timer (60ms)
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
                match SystemTray::new(on_tray_action_retry.clone()) {
                    Ok(t) => {
                        info!("System tray successfully initialized on retry!");
                        *tray_for_timer.borrow_mut() = Some(t);
                    }
                    Err(e) => {
                        warn!("Tray retry unavailable: {:?}", e);
                    }
                }
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
        let y = (screen_h - 360) / 2;
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
    }

    info!("Step 9: Calling slint::run_event_loop_until_quit()...");
    let run_res = slint::run_event_loop_until_quit();
    info!("Step 10: Event loop exited with result: {:?}", run_res);
    monitor.stop();

    #[cfg(windows)]
    {
        let _ = crate::telemetry::driver::stop_service();
    }

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
        win.on_save_settings(move |_mode, _auto, _int, _lang, _anim, _src, _vid, _pid, _smoothing| {
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
            false,
            "core0".into(),
            "1A86".into(),
            "E317".into(),
            1,
        );
        // Test Activation Mode and Driver Callbacks
        let driver_req = Arc::new(AtomicBool::new(false));
        let driver_req_clone = Arc::clone(&driver_req);
        win.on_request_install_driver(move || {
            driver_req_clone.store(true, Ordering::SeqCst);
        });

        let finish_act = Arc::new(AtomicBool::new(false));
        let finish_act_clone = Arc::clone(&finish_act);
        win.on_finish_activation(move || {
            finish_act_clone.store(true, Ordering::SeqCst);
        });

        let lang_changed = Arc::new(AtomicBool::new(false));
        let lang_changed_clone = Arc::clone(&lang_changed);
        win.on_change_language(move |_lang| {
            lang_changed_clone.store(true, Ordering::SeqCst);
        });

        win.set_is_activation_mode(true);
        assert!(win.get_is_activation_mode(), "Must be in activation mode");
        win.set_is_driver_installed(false);
        assert!(!win.get_is_driver_installed(), "Driver must be reported as uninstalled");

        win.invoke_request_install_driver();
        assert!(driver_req.load(Ordering::SeqCst), "request_install_driver must fire");

        win.invoke_change_language("zh".into());
        assert!(lang_changed.load(Ordering::SeqCst), "change_language must fire");

        win.invoke_finish_activation();
        assert!(finish_act.load(Ordering::SeqCst), "finish_activation must fire");

        win.set_is_activation_mode(false);
        win.set_is_driver_installed(true);
        assert!(!win.get_is_activation_mode());
        assert!(win.get_is_driver_installed());

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
            win.set_cpu_temp(35 + (i % 30));
            win.set_cpu_load(i % 100);
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
