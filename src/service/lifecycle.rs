use crate::config::AppConfig;
use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn pid_file_path() -> PathBuf {
    AppConfig::config_dir().join("daemon.pid")
}

pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    unsafe {
        // Sending signal 0 performs error checking without sending a signal
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle != 0 {
            CloseHandle(handle);
            true
        } else {
            false
        }
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        false
    }
}

pub fn get_daemon_pid() -> Option<u32> {
    let path = pid_file_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    return Some(pid);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
    None
}

pub fn write_daemon_pid(pid: u32) {
    let path = pid_file_path();
    let _ = fs::write(path, pid.to_string());
}

pub fn remove_daemon_pid() {
    let path = pid_file_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

pub fn stop_daemon() -> bool {
    if let Some(pid) = get_daemon_pid() {
        info!("Stopping background daemon with PID {}...", pid);
        #[cfg(target_os = "linux")]
        unsafe {
            let _ = libc::kill(pid as i32, libc::SIGTERM);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle != 0 {
                TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }

        // Wait up to 600 ms for daemon to exit cleanly
        for _ in 0..12 {
            std::thread::sleep(Duration::from_millis(50));
            if !is_process_alive(pid) {
                remove_daemon_pid();
                return true;
            }
        }
        remove_daemon_pid();
        return true;
    }
    false
}

pub fn spawn_daemon() -> bool {
    if get_daemon_pid().is_some() {
        return true; // Already running
    }

    if let Ok(current_exe) = std::env::current_exe() {
        let mut cmd = Command::new(current_exe);
        cmd.arg("--daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("Spawned background daemon with PID {}", child.id());
                // Give it a brief moment to initialize
                std::thread::sleep(Duration::from_millis(150));
                true
            }
            Err(e) => {
                warn!("Failed to spawn background daemon: {e}");
                false
            }
        }
    } else {
        false
    }
}
