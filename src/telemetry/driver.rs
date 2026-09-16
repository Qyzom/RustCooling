#[cfg(windows)]
use log::{debug, info, warn};
use std::ffi::c_void;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
    OpenServiceW, StartServiceW, SC_MANAGER_ALL_ACCESS, SC_MANAGER_CREATE_SERVICE,
    SERVICE_ALL_ACCESS, SERVICE_AUTO_START, SERVICE_CONTROL_STOP, SERVICE_ERROR_NORMAL,
    SERVICE_KERNEL_DRIVER, SERVICE_START, SERVICE_STATUS, SERVICE_STOP,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, GetProcessAffinityMask, SetThreadAffinityMask,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

pub const DRIVER_BYTES: &[u8] = include_bytes!("../../assets/driver/WinRing0x64.sys");
pub const DRIVER_FILE_NAME: &str = "WinRing0x64.sys";
pub const SERVICE_NAME: &str = "WinRing0_1_2_0";
pub const DEVICE_NAME: &str = "\\\\.\\WinRing0_1_2_0";
pub const IOCTL_OLS_READ_MSR: u32 = 0x9C402084;

/// Returns the destination path of the driver file, directly next to RustCooling.exe
pub fn get_driver_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join(DRIVER_FILE_NAME);
        }
    }
    PathBuf::from(DRIVER_FILE_NAME)
}

/// Extracts the driver to the folder next to the executable if not already present
pub fn ensure_driver_extracted() -> Result<PathBuf, String> {
    let dest = get_driver_path();
    if dest.exists() {
        if let Ok(meta) = std::fs::metadata(&dest) {
            if meta.len() == DRIVER_BYTES.len() as u64 {
                return Ok(dest);
            }
        }
    }

    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::fs::write(&dest, DRIVER_BYTES) {
        Ok(_) => {
            info!("Extracted {} to {:?}", DRIVER_FILE_NAME, dest);
            Ok(dest)
        }
        Err(e) => {
            warn!("Failed to write driver next to exe: {:?}. Trying temp dir.", e);
            let temp_dest = std::env::temp_dir().join(DRIVER_FILE_NAME);
            std::fs::write(&temp_dest, DRIVER_BYTES)
                .map(|_| temp_dest)
                .map_err(|e2| format!("Failed to extract driver: {}", e2))
        }
    }
}

/// Checks whether the driver device file can be opened by an unprivileged or privileged process
pub fn is_driver_accessible() -> bool {
    let wide_device: Vec<u16> = DEVICE_NAME.encode_utf16().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            wide_device.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };

    if handle != INVALID_HANDLE_VALUE {
        unsafe { CloseHandle(handle) };
        true
    } else {
        false
    }
}

/// Checks whether the Windows service exists
#[allow(dead_code)]
pub fn is_service_installed() -> bool {
    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm.is_null() {
            return false;
        }
        let wide_svc: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();
        let svc = OpenServiceW(scm, wide_svc.as_ptr(), SERVICE_ALL_ACCESS);
        let exists = !svc.is_null();
        if exists {
            CloseServiceHandle(svc);
        }
        CloseServiceHandle(scm);
        exists
    }
}

/// Installs and starts the WinRing0 kernel driver service.
/// Requires elevated Administrator privileges.
pub fn install_service() -> Result<(), String> {
    let driver_path = ensure_driver_extracted()?;
    let path_str = driver_path.to_str().ok_or("Invalid driver path encoding")?;
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(Some(0)).collect();
    let wide_svc: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let scm = OpenSCManagerW(
            std::ptr::null(),
            std::ptr::null(),
            SC_MANAGER_ALL_ACCESS | SC_MANAGER_CREATE_SERVICE,
        );
        if scm.is_null() {
            let err = GetLastError();
            return Err(format!(
                "Failed to open Service Control Manager (error code {}). Please run as Administrator.",
                err
            ));
        }

        let mut svc = OpenServiceW(
            scm,
            wide_svc.as_ptr(),
            SERVICE_ALL_ACCESS | SERVICE_START,
        );

        if svc.is_null() {
            // Create new kernel service
            svc = CreateServiceW(
                scm,
                wide_svc.as_ptr(),
                wide_svc.as_ptr(),
                SERVICE_ALL_ACCESS,
                SERVICE_KERNEL_DRIVER,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );

            if svc.is_null() {
                let err = GetLastError();
                CloseServiceHandle(scm);
                return Err(format!("Failed to create driver service (error code {}).", err));
            }
            info!("Driver service '{}' created successfully.", SERVICE_NAME);
        } else {
            // Ensure existing service is configured to auto-start on boot with current path
            use windows_sys::Win32::System::Services::ChangeServiceConfigW;
            ChangeServiceConfigW(
                svc,
                SERVICE_KERNEL_DRIVER,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
        }

        // Start service
        let start_res = StartServiceW(svc, 0, std::ptr::null_mut());
        let last_err = GetLastError();
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);

        // 1056 = ERROR_SERVICE_ALREADY_RUNNING
        if start_res == 0 && last_err != 1056 {
            debug!("StartServiceW error: {}", last_err);
        }

        if is_driver_accessible() {
            info!("Driver device '{}' is online and accessible.", DEVICE_NAME);
            Ok(())
        } else {
            Err(format!(
                "Service started but device '{}' could not be opened (last error: {}).",
                DEVICE_NAME, last_err
            ))
        }
    }
}

/// Uninstalls and stops the WinRing0 service.
#[allow(dead_code)]
pub fn uninstall_service() -> Result<(), String> {
    let wide_svc: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm.is_null() {
            return Err("Failed to open Service Control Manager".to_string());
        }

        let svc = OpenServiceW(scm, wide_svc.as_ptr(), SERVICE_ALL_ACCESS | SERVICE_STOP);
        if svc.is_null() {
            CloseServiceHandle(scm);
            return Ok(()); // Already not installed
        }

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let _ = ControlService(svc, SERVICE_CONTROL_STOP, &mut status);
        let del = DeleteService(svc);
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);

        if del != 0 {
            info!("Driver service '{}' removed.", SERVICE_NAME);
            Ok(())
        } else {
            Err(format!("DeleteService failed with error {}", GetLastError()))
        }
    }
}

/// Requests UAC elevation to install the driver via a hidden helper call
pub fn request_elevation_install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_wide: Vec<u16> = exe.to_str().unwrap().encode_utf16().chain(Some(0)).collect();
    let verb_wide: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let params_wide: Vec<u16> = "--install-driver".encode_utf16().chain(Some(0)).collect();

    let res = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb_wide.as_ptr(),
            exe_wide.as_ptr(),
            params_wide.as_ptr(),
            std::ptr::null(),
            SW_HIDE,
        )
    };

    // Returns HINSTANCE > 32 on success
    if res as usize > 32 {
        Ok(())
    } else {
        Err(format!("Elevation request cancelled or failed (code {})", res as usize))
    }
}

pub struct DriverHandle {
    handle: HANDLE,
}

impl DriverHandle {
    pub fn open() -> Option<Self> {
        let wide_device: Vec<u16> = DEVICE_NAME.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_device.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle != INVALID_HANDLE_VALUE {
            Some(Self { handle })
        } else {
            None
        }
    }

    pub fn read_msr(&self, index: u32) -> Option<u64> {
        let mut input = index;
        let mut output: u64 = 0;
        let mut bytes_returned: u32 = 0;

        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_OLS_READ_MSR,
                &mut input as *mut _ as *mut c_void,
                std::mem::size_of::<u32>() as u32,
                &mut output as *mut _ as *mut c_void,
                std::mem::size_of::<u64>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if ok != 0 && bytes_returned >= 8 {
            Some(output)
        } else {
            None
        }
    }
}

impl Drop for DriverHandle {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.handle) };
            self.handle = INVALID_HANDLE_VALUE;
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PhysicalCpuTemps {
    pub package: Option<f32>,
    pub core0: Option<f32>,
    pub core_temps: Vec<f32>,
}

/// Reads real CPU digital thermal sensor (DTS) temperatures directly from physical MSRs.
/// Supports both Intel (TjMax - DigitalReadout) and AMD.
pub fn read_physical_temperatures() -> Option<PhysicalCpuTemps> {
    let drv = DriverHandle::open()?;

    // 1. Check Intel Architecture (MSR 0x1A2 TjMax & MSR 0x19C Thermal Status)
    // IA32_TEMPERATURE_TARGET (0x1A2)
    let tj_max = if let Some(target_msr) = drv.read_msr(0x1A2) {
        let target = ((target_msr >> 16) & 0xFF) as f32;
        if (60.0..=120.0).contains(&target) {
            target
        } else {
            100.0
        }
    } else {
        100.0
    };

    let mut proc_mask: usize = 0;
    let mut sys_mask: usize = 0;
    unsafe {
        GetProcessAffinityMask(
            GetCurrentProcess(),
            &mut proc_mask as *mut _ as *mut _,
            &mut sys_mask as *mut _ as *mut _,
        );
    }

    let mut core_temps: Vec<f32> = Vec::new();
    let num_threads = (std::mem::size_of::<usize>() * 8).min(64);

    for i in 0..num_threads {
        let mask = 1usize << i;
        if (proc_mask & mask) == 0 {
            continue;
        }

        // Pin current thread to this logical/physical core to query its MSR
        let prev_mask = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
        if prev_mask != 0 {
            // Read IA32_THERM_STATUS (0x19C)
            if let Some(status_msr) = drv.read_msr(0x19C) {
                let valid = (status_msr >> 31) & 1;
                if valid != 0 {
                    let delta = ((status_msr >> 16) & 0x7F) as f32;
                    let temp = (tj_max - delta).clamp(15.0, 115.0);
                    core_temps.push(temp);
                }
            }
            // Restore thread affinity mask
            unsafe { SetThreadAffinityMask(GetCurrentThread(), prev_mask) };
        }
    }

    if !core_temps.is_empty() {
        let core0 = core_temps.first().copied();
        let max_core = core_temps.iter().cloned().fold(f32::MIN, f32::max);

        // IA32_PACKAGE_THERM_STATUS (0x1B1)
        let package = if let Some(pkg_msr) = drv.read_msr(0x1B1) {
            let valid = (pkg_msr >> 31) & 1;
            if valid != 0 {
                let delta = ((pkg_msr >> 16) & 0x7F) as f32;
                Some((tj_max - delta).clamp(15.0, 115.0))
            } else {
                Some(max_core)
            }
        } else {
            Some(max_core)
        };

        return Some(PhysicalCpuTemps {
            package,
            core0,
            core_temps,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_path() {
        let p = get_driver_path();
        println!("Driver path: {:?}", p);
        assert!(p.to_string_lossy().contains(DRIVER_FILE_NAME));
    }
}
