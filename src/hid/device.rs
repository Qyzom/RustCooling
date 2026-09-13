use crate::protocol::{build_windows_report, Command};
#[cfg(not(windows))]
use crate::protocol::build_frame;
use hidapi::{HidApi, HidDevice};
use std::sync::{Arc, Mutex};
use log::{debug, info, warn};

pub const VENDOR_ID: u16 = 0x1A86;
pub const PRODUCT_ID: u16 = 0xE317;

pub struct DeviceManager {
    hid_api: Arc<Mutex<Option<HidApi>>>,
    device: Arc<Mutex<Option<HidDevice>>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            hid_api: Arc::new(Mutex::new(None)),
            device: Arc::new(Mutex::new(None)),
        }
    }

    fn init_api(&self) -> Result<(), String> {
        let mut api_guard = self.hid_api.lock().map_err(|e| e.to_string())?;
        if api_guard.is_none() {
            let api = HidApi::new().map_err(|e| format!("Failed to initialize HidApi: {e}"))?;
            *api_guard = Some(api);
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        if let Ok(guard) = self.device.lock() {
            guard.is_some()
        } else {
            false
        }
    }

    pub fn open_device(&self) -> bool {
        self.close_device();

        if let Err(e) = self.init_api() {
            warn!("HID Init API error: {e}");
            return false;
        }

        let mut api_guard = match self.hid_api.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        if let Some(ref mut api) = *api_guard {
            // Refresh device list
            if let Err(e) = api.refresh_devices() {
                warn!("HID refresh error: {e}");
            }

            match api.open(VENDOR_ID, PRODUCT_ID) {
                Ok(dev) => {
                    info!("Successfully connected to ID-COOLING LCD Display (VID: 0x{:04X}, PID: 0x{:04X})", VENDOR_ID, PRODUCT_ID);
                    if let Ok(mut dev_guard) = self.device.lock() {
                        *dev_guard = Some(dev);
                    }
                    true
                }
                Err(e) => {
                    debug!("Display device not found or busy: {e}");
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn close_device(&self) {
        if let Ok(mut guard) = self.device.lock() {
            *guard = None;
        }
    }

    pub fn send_command(&self, command: Command, value: u16) -> bool {
        let mut dev_guard = match self.device.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        if let Some(ref dev) = *dev_guard {
            #[cfg(windows)]
            let payload = build_windows_report(command, value);
            #[cfg(not(windows))]
            let payload = build_frame(command, value);

            match dev.write(&payload) {
                Ok(written) if written == payload.len() => {
                    debug!("Sent command {:?} with value {}", command, value);
                    true
                }
                Ok(written) => {
                    warn!("Incomplete HID write: {} / {} bytes. Reconnecting...", written, payload.len());
                    *dev_guard = None;
                    false
                }
                Err(e) => {
                    warn!("HID write failed: {e}. Reconnecting...");
                    *dev_guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn send_temperature(&self, temp_c: u16) -> bool {
        self.send_command(Command::Temperature, temp_c)
    }

    pub fn send_frequency(&self, freq_mhz: u16) -> bool {
        self.send_command(Command::Frequency, freq_mhz)
    }

    pub fn send_usage(&self, percent: u16) -> bool {
        self.send_command(Command::Usage, percent)
    }

    pub fn send_show(&self, show: bool) -> bool {
        self.send_command(Command::Show, if show { 1 } else { 0 })
    }
}

impl Drop for DeviceManager {
    fn drop(&mut self) {
        let _ = self.send_show(false);
        self.close_device();
    }
}

