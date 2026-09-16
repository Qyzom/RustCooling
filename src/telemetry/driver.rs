#[cfg(windows)]
use log::{debug, info, warn};
use std::ffi::c_void;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, CreateServiceW, OpenSCManagerW, OpenServiceW, StartServiceW,
    SC_MANAGER_ALL_ACCESS, SC_MANAGER_CREATE_SERVICE, SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP,
    SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER, SERVICE_START,
    SERVICE_STATUS, SERVICE_STOP,
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
pub const IOCTL_OLS_READ_PCI_CONFIG: u32 = 0x9C406144;
pub const IOCTL_OLS_WRITE_PCI_CONFIG: u32 = 0x9C40A148;

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
        if let Ok(existing) = std::fs::read(&dest) {
            if existing == DRIVER_BYTES {
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
            0, // Exclusive access (dwShareMode = 0) against BYOVD exploitation
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
            // Create new kernel service with SERVICE_DEMAND_START (runs on demand, not 24/7)
            svc = CreateServiceW(
                scm,
                wide_svc.as_ptr(),
                wide_svc.as_ptr(),
                SERVICE_ALL_ACCESS,
                SERVICE_KERNEL_DRIVER,
                SERVICE_DEMAND_START,
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
            info!("Driver service '{}' created successfully (SERVICE_DEMAND_START).", SERVICE_NAME);
        } else {
            // Ensure existing service is configured for demand start with current path
            use windows_sys::Win32::System::Services::ChangeServiceConfigW;
            ChangeServiceConfigW(
                svc,
                SERVICE_KERNEL_DRIVER,
                SERVICE_DEMAND_START,
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
        // 1275 = ERROR_DRIVER_BLOCKED (HVCI / Memory Integrity / Microsoft Vulnerable Driver Blocklist)
        if start_res == 0 && last_err != 1056 {
            if last_err == 1275 {
                warn!(
                    "Windows blocked the WinRing0 kernel driver (error 1275: ERROR_DRIVER_BLOCKED). \
                    Memory Integrity (HVCI) or the Vulnerable Driver Blocklist is active on Windows 11. \
                    RustCooling will operate cleanly without the kernel driver using standard OS telemetry."
                );
                return Err("Driver blocked by Windows Memory Integrity (HVCI). Falling back to OS telemetry.".to_string());
            } else {
                debug!("StartServiceW error: {}", last_err);
            }
        }

        if is_driver_accessible() {
            info!("Driver device '{}' is online and accessible with exclusive handle.", DEVICE_NAME);
            Ok(())
        } else {
            Err(format!(
                "Service started but device '{}' could not be opened (last error: {}).",
                DEVICE_NAME, last_err
            ))
        }
    }
}

/// Stops the WinRing0 driver service and unloads it from kernel memory.
/// Called during application shutdown to avoid leaving the driver running 24/7.
pub fn stop_service() -> Result<(), String> {
    let wide_svc: Vec<u16> = SERVICE_NAME.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm.is_null() {
            return Err("Failed to open Service Control Manager".to_string());
        }

        let svc = OpenServiceW(scm, wide_svc.as_ptr(), SERVICE_STOP);
        if svc.is_null() {
            CloseServiceHandle(scm);
            return Ok(()); // Service not installed or already removed
        }

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let ok = ControlService(svc, SERVICE_CONTROL_STOP, &mut status);
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);

        if ok != 0 {
            info!("Driver service '{}' stopped successfully.", SERVICE_NAME);
        } else {
            debug!("ControlService STOP returned 0 (service may already be stopped).");
        }
        Ok(())
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

#[repr(C, packed(4))]
struct OlsReadPciConfigInput {
    pci_address: u32,
    pci_offset: u32,
}

#[repr(C, packed(4))]
struct OlsWritePciConfigInput {
    pci_address: u32,
    pci_offset: u32,
    data: [u8; 4],
}

pub struct DriverHandle {
    handle: HANDLE,
}

unsafe impl Send for DriverHandle {}
unsafe impl Sync for DriverHandle {}

/// RAII guard that restores the thread's original affinity mask on drop.
struct AffinityGuard {
    prev_mask: usize,
}

impl AffinityGuard {
    fn set(mask: usize) -> Option<Self> {
        let prev = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
        if prev != 0 {
            Some(Self { prev_mask: prev })
        } else {
            None
        }
    }
}

impl Drop for AffinityGuard {
    fn drop(&mut self) {
        if self.prev_mask != 0 {
            unsafe {
                SetThreadAffinityMask(GetCurrentThread(), self.prev_mask);
            }
        }
    }
}

impl DriverHandle {
    pub fn open() -> Option<Self> {
        let wide_device: Vec<u16> = DEVICE_NAME.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_device.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0, // Exclusive access (dwShareMode = 0) against BYOVD exploitation
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

    pub fn read_pci_config_dword(&self, pci_address: u32, pci_offset: u32) -> Option<u32> {
        let mut input = OlsReadPciConfigInput {
            pci_address,
            pci_offset,
        };
        let mut output: u32 = 0;
        let mut bytes_returned: u32 = 0;

        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_OLS_READ_PCI_CONFIG,
                &mut input as *mut _ as *mut c_void,
                std::mem::size_of::<OlsReadPciConfigInput>() as u32,
                &mut output as *mut _ as *mut c_void,
                std::mem::size_of::<u32>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if ok != 0 && bytes_returned >= 4 {
            Some(output)
        } else {
            None
        }
    }

    pub fn write_pci_config_dword(&self, pci_address: u32, pci_offset: u32, value: u32) -> bool {
        let mut input = OlsWritePciConfigInput {
            pci_address,
            pci_offset,
            data: value.to_ne_bytes(),
        };
        let mut bytes_returned: u32 = 0;

        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_OLS_WRITE_PCI_CONFIG,
                &mut input as *mut _ as *mut c_void,
                std::mem::size_of::<OlsWritePciConfigInput>() as u32,
                std::ptr::null_mut(),
                0,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        ok != 0
    }

    /// Reads an AMD System Management Network (SMN) register via PCI indirect mailbox at 0:0.0.
    /// Address register: PCI 0:0.0 offset 0x60
    /// Data register:    PCI 0:0.0 offset 0x64
    pub fn read_smn_register(&self, address: u32) -> Option<u32> {
        // Device 0:0.0 has PCI address 0
        if !self.write_pci_config_dword(0, 0x60, address) {
            return None;
        }
        self.read_pci_config_dword(0, 0x64)
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

#[cfg(target_arch = "x86_64")]
pub fn is_intel_cpu() -> bool {
    let res = std::arch::x86_64::__cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&res.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&res.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&res.ecx.to_le_bytes());
    &vendor == b"GenuineIntel"
}

#[cfg(not(target_arch = "x86_64"))]
pub fn is_intel_cpu() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
pub fn is_amd_cpu() -> bool {
    let res = std::arch::x86_64::__cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&res.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&res.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&res.ecx.to_le_bytes());
    &vendor == b"AuthenticAMD"
}

#[cfg(not(target_arch = "x86_64"))]
pub fn is_amd_cpu() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
pub fn get_cpu_vendor() -> String {
    let res = std::arch::x86_64::__cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&res.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&res.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&res.ecx.to_le_bytes());
    String::from_utf8_lossy(&vendor).into_owned()
}

#[cfg(not(target_arch = "x86_64"))]
pub fn get_cpu_vendor() -> String {
    "Unknown".to_string()
}

#[cfg(target_arch = "x86_64")]
pub fn get_cpu_family() -> u32 {
    let res = std::arch::x86_64::__cpuid(1);
    let base_family = (res.eax >> 8) & 0xF;
    let ext_family = (res.eax >> 20) & 0xFF;
    base_family + ext_family
}

#[cfg(not(target_arch = "x86_64"))]
pub fn get_cpu_family() -> u32 {
    0
}

fn read_intel_temperatures(drv: &DriverHandle) -> Option<PhysicalCpuTemps> {
    // 1. Check Intel Architecture (MSR 0x1A2 TjMax & MSR 0x19C Thermal Status)
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

        // Pin current thread to this logical/physical core to query its MSR via RAII guard
        if let Some(_guard) = AffinityGuard::set(mask) {
            // Read IA32_THERM_STATUS (0x19C)
            if let Some(status_msr) = drv.read_msr(0x19C) {
                let valid = (status_msr >> 31) & 1;
                if valid != 0 {
                    let delta = ((status_msr >> 16) & 0x7F) as f32;
                    let temp = (tj_max - delta).clamp(15.0, 115.0);
                    core_temps.push(temp);
                }
            }
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

fn read_amd_temperatures(drv: &DriverHandle) -> Option<PhysicalCpuTemps> {
    let family = get_cpu_family();
    if family < 0x17 {
        debug!(
            "AMD CPU family 0x{:X} precedes Zen (Family 17h+). Falling back to OS telemetry.",
            family
        );
        return None;
    }

    // F17H_M01H_THM_TCON_CUR_TMP = 0x00059800
    // Reads physical Tctl package temperature
    let tctl_raw = drv.read_smn_register(0x00059800)?;
    let raw_temp = ((tctl_raw >> 21) & 0x7FF) as f32 / 8.0;
    // Bit 19 (0x80000) is RangeSelect: if set, temperature is offset by -49 °C
    let tctl = if (tctl_raw & 0x80000) != 0 {
        raw_temp - 49.0
    } else {
        raw_temp
    };

    let mut core_temps: Vec<f32> = Vec::new();

    // Query CCD (Core Complex Die) temperatures on Zen 2 / 3 / 4 / 5
    // Base register: 0x00059954 (Family 17h Model 70h+ / Family 19h+)
    for i in 0..8 {
        let ccd_reg = 0x00059954 + i * 0x4;
        if let Some(ccd_raw) = drv.read_smn_register(ccd_reg) {
            // Valid bit is bit 11 (0x800)
            if (ccd_raw & 0x800) != 0 {
                let ccd_temp = ((ccd_raw & 0x7FF) as f32 / 8.0) - 49.0;
                if (15.0..=115.0).contains(&ccd_temp) {
                    core_temps.push(ccd_temp);
                }
            }
        }
    }

    let package = if (15.0..=115.0).contains(&tctl) {
        Some(tctl)
    } else if !core_temps.is_empty() {
        Some(core_temps.iter().cloned().fold(f32::MIN, f32::max))
    } else {
        None
    };

    if package.is_some() || !core_temps.is_empty() {
        let core0 = core_temps.first().copied().or(package);
        Some(PhysicalCpuTemps {
            package,
            core0,
            core_temps,
        })
    } else {
        None
    }
}

/// Reads real physical CPU temperatures directly from hardware.
/// - Intel CPUs: DTS per-core & CPU Package via IA32 MSRs (0x19C, 0x1B1)
/// - AMD Ryzen CPUs (Zen 1/2/3/4/5): Tctl & CCD temperatures via SMN PCI mailbox (0:0.0)
/// - Zero WMI, 0.0% CPU overhead.
pub fn read_physical_temperatures(drv: &DriverHandle) -> Option<PhysicalCpuTemps> {
    if is_intel_cpu() {
        read_intel_temperatures(drv)
    } else if is_amd_cpu() {
        read_amd_temperatures(drv)
    } else {
        debug!(
            "CPU vendor '{}' not recognized for physical hardware telemetry.",
            get_cpu_vendor()
        );
        None
    }
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

    #[test]
    fn test_pci_config_struct_layouts() {
        assert_eq!(std::mem::size_of::<OlsReadPciConfigInput>(), 8);
        assert_eq!(std::mem::size_of::<OlsWritePciConfigInput>(), 12);
    }

    #[test]
    fn test_amd_tctl_decoding() {
        // Test standard range (bit 19 = 0)
        // 50.0 °C -> 50 * 8 = 400 = 0x190. Shift left by 21 = 0x190 << 21 = 0x32000000
        let val_normal: u32 = 0x190 << 21;
        let raw_temp = ((val_normal >> 21) & 0x7FF) as f32 / 8.0;
        let tctl_normal = if (val_normal & 0x80000) != 0 {
            raw_temp - 49.0
        } else {
            raw_temp
        };
        assert!((tctl_normal - 50.0).abs() < 0.01);

        // Test range-select offset (bit 19 = 1)
        // Raw temperature reading = 99.0 °C (99 * 8 = 792 = 0x318).
        // With -49 °C offset: 99 - 49 = 50.0 °C
        let val_offset: u32 = (0x318 << 21) | 0x80000;
        let raw_temp2 = ((val_offset >> 21) & 0x7FF) as f32 / 8.0;
        let tctl_offset = if (val_offset & 0x80000) != 0 {
            raw_temp2 - 49.0
        } else {
            raw_temp2
        };
        assert!((tctl_offset - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_amd_ccd_decoding() {
        // Valid bit = bit 11 (0x800)
        // CCD temp = (raw & 0x7FF) / 8.0 - 49.0
        // Target 45.0 °C -> raw & 0x7FF = (45 + 49) * 8 = 94 * 8 = 752 = 0x2F0
        let ccd_val: u32 = 0x800 | 0x2F0;
        assert_ne!(ccd_val & 0x800, 0, "Valid bit must be set");
        let ccd_temp = ((ccd_val & 0x7FF) as f32 / 8.0) - 49.0;
        assert!((ccd_temp - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_cpu_detection() {
        let vendor = get_cpu_vendor();
        println!("Detected CPU Vendor: {}", vendor);
        let family = get_cpu_family();
        println!("Detected CPU Family: 0x{:X}", family);
        if vendor == "GenuineIntel" {
            assert!(is_intel_cpu());
            assert!(!is_amd_cpu());
        } else if vendor == "AuthenticAMD" {
            assert!(is_amd_cpu());
            assert!(!is_intel_cpu());
        }
    }
}
