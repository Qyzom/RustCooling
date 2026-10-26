use log::info;

#[cfg(windows)]
use auto_launch::AutoLaunchBuilder;

pub fn get_executable_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        // When running as an AppImage, std::env::current_exe() points to the temporary mount point (/tmp/.mount_RustCoXXXX/usr/bin/RustCooling)
        // which disappears after shutdown. $APPIMAGE contains the real, permanent path on disk.
        if let Ok(appimage_path) = std::env::var("APPIMAGE") {
            if !appimage_path.is_empty() {
                return Some(std::path::PathBuf::from(appimage_path));
            }
        }
    }
    std::env::current_exe().ok()
}

pub fn set_autostart_mode(mode: &str) {
    let mode = mode.to_lowercase();
    #[cfg(windows)]
    {
        if let Some(current_exe) = get_executable_path() {
            let app_name = "RustCooling";
            let current_exe_str = current_exe.to_string_lossy();
            let args: &[&str] = if mode == "daemon" {
                &["--daemon"]
            } else {
                &["--minimized"]
            };
            let auto = AutoLaunchBuilder::new()
                .set_app_name(app_name)
                .set_app_path(&current_exe_str)
                .set_args(args)
                .build();

            if let Ok(auto) = auto {
                if mode != "none" {
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

            if mode != "none" {
                if let Some(current_exe) = get_executable_path() {
                    let _ = std::fs::create_dir_all(&autostart_dir);
                    let exe_path = current_exe.to_string_lossy();
                    let exec_cmd = if mode == "daemon" {
                        format!("\"{}\" --daemon", exe_path)
                    } else {
                        format!("\"{}\" --minimized", exe_path)
                    };
                    let desktop_entry = format!(
                        "[Desktop Entry]\n\
                         Type=Application\n\
                         Version=1.0\n\
                         Name=RustCooling\n\
                         Comment=ID-COOLING FX LCD Controller\n\
                         Icon=rustcooling\n\
                         Exec={}\n\
                         Terminal=false\n\
                         Categories=Utility;HardwareSettings;\n\
                         StartupNotify=false\n\
                         StartupWMClass=RustCooling\n",
                        exec_cmd
                    );
                    if let Ok(existing) = std::fs::read_to_string(&desktop_file) {
                        if existing == desktop_entry {
                            return;
                        }
                    }
                    let _ = std::fs::write(&desktop_file, desktop_entry);
                    info!(
                        "Linux autostart enabled with mode '{}': created {:?}",
                        mode, desktop_file
                    );
                }
            } else if desktop_file.exists() {
                let _ = std::fs::remove_file(&desktop_file);
                info!("Linux autostart disabled: removed {:?}", desktop_file);
            }
        }
    }
}

pub fn set_autostart(enable: bool) {
    set_autostart_mode(if enable { "gui" } else { "none" });
}

pub fn is_autostart_registered() -> bool {
    #[cfg(windows)]
    {
        if let Some(current_exe) = get_executable_path() {
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
