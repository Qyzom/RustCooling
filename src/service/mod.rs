use crate::config::AppConfig;
use crate::hid::DeviceManager;
use crate::telemetry::{create_telemetry_provider, CpuMetrics};
use log::{debug, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct MonitorState {
    pub metrics: Arc<Mutex<CpuMetrics>>,
    pub is_connected: Arc<AtomicBool>,
    pub broadcast_label: Arc<Mutex<String>>,
    pub broadcast_value: Arc<Mutex<String>>,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(CpuMetrics::default())),
            is_connected: Arc::new(AtomicBool::new(false)),
            broadcast_label: Arc::new(Mutex::new(crate::i18n::I18n::get().metric_temperature)),
            broadcast_value: Arc::new(Mutex::new("—".to_string())),
        }
    }
}

pub struct MonitorService {
    config: Arc<Mutex<AppConfig>>,
    state: MonitorState,
    running: Arc<AtomicBool>,
    device: Arc<DeviceManager>,
    worker_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl MonitorService {
    pub fn new(config: Arc<Mutex<AppConfig>>) -> Self {
        Self {
            config,
            state: MonitorState::new(),
            running: Arc::new(AtomicBool::new(false)),
            device: Arc::new(DeviceManager::new()),
            worker_handle: Mutex::new(None),
        }
    }

    pub fn get_state(&self) -> MonitorState {
        self.state.clone()
    }

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = Arc::clone(&self.running);
        let config = Arc::clone(&self.config);
        let state = self.state.clone();
        let device = Arc::clone(&self.device);

        let handle = thread::spawn(move || {
            #[cfg(windows)]
            unsafe {
                use windows_sys::Win32::System::Threading::{
                    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
                };
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            }
            info!("Monitor background service started with high priority.");
            let mut telemetry = create_telemetry_provider();
            let mut was_connected = false;
            let mut active_vid = 0u16;
            let mut active_pid = 0u16;
            let mut last_displayed_val: Option<u16> = None;
            let mut last_display_mode = String::new();
            let mut smoothed_temp: Option<f32> = None;

            while running.load(Ordering::Relaxed) {
                // Read current display configuration
                let (
                    display_mode,
                    interval_ms,
                    temp_source,
                    temp_smoothing,
                    custom_vid,
                    custom_pid,
                ) = {
                    let cfg = config.lock().unwrap();
                    (
                        cfg.display_mode.clone(),
                        cfg.update_interval_ms.clamp(100, 3000),
                        cfg.temp_source.clone(),
                        cfg.temp_smoothing.min(5),
                        cfg.custom_vid,
                        cfg.custom_pid,
                    )
                };

                // Reset smoothing state if mode changed
                if display_mode != last_display_mode {
                    last_displayed_val = None;
                    smoothed_temp = None;
                    last_display_mode = display_mode.clone();
                }

                // Reconnect if VID/PID changed
                if active_vid != 0 && (custom_vid != active_vid || custom_pid != active_pid) {
                    info!(
                        "VID/PID configuration changed: 0x{:04X}:0x{:04X} -> 0x{:04X}:0x{:04X}",
                        active_vid, active_pid, custom_vid, custom_pid
                    );
                    device.close_device();
                    state.is_connected.store(false, Ordering::SeqCst);
                    smoothed_temp = None;
                }
                active_vid = custom_vid;
                active_pid = custom_pid;

                // Ensure device is connected
                if !device.is_connected() {
                    if device.open_device(custom_vid, custom_pid) {
                        info!(
                            "LCD Display connected (VID: 0x{:04X}, PID: 0x{:04X}).",
                            custom_vid, custom_pid
                        );
                        let _ = device.send_show(true);
                        state.is_connected.store(true, Ordering::SeqCst);
                        was_connected = true;
                    } else {
                        if was_connected {
                            warn!("LCD Display disconnected. Retrying in 2 seconds...");
                            state.is_connected.store(false, Ordering::SeqCst);
                            was_connected = false;
                        }
                        thread::sleep(Duration::from_millis(2000));
                        continue;
                    }
                }

                // Update Telemetry with chosen temp source
                telemetry.set_temp_source(&temp_source);
                telemetry.update();
                let metrics = telemetry.get_metrics();

                if let Ok(mut m) = state.metrics.lock() {
                    *m = metrics.clone();
                }

                // Pure mathematical EMA smoothing filter with deadband
                let calc_smoothed_temp = |raw_temp: f32,
                                          smoothed: &mut Option<f32>,
                                          last_val: Option<u16>,
                                          smoothing: u32|
                 -> u16 {
                    if smoothing == 0 {
                        *smoothed = Some(raw_temp);
                        raw_temp.round().clamp(0.0, 199.0) as u16
                    } else {
                        let alpha = 1.0 / (1.0 + smoothing as f32 * 0.4);
                        let new_val = match *smoothed {
                            Some(prev) => prev * (1.0 - alpha) + raw_temp * alpha,
                            None => raw_temp,
                        };
                        *smoothed = Some(new_val);
                        let rounded = new_val.round().clamp(0.0, 199.0) as u16;

                        if let Some(prev) = last_val {
                            if (rounded as i32 - prev as i32).abs() < smoothing as i32 {
                                prev
                            } else {
                                rounded
                            }
                        } else {
                            rounded
                        }
                    }
                };

                // Dispatch strictly one HID packet per update interval
                match display_mode.as_str() {
                    "load" => {
                        if let Ok(mut lbl) = state.broadcast_label.lock() {
                            *lbl = crate::i18n::I18n::get().metric_load;
                        }
                        if let Some(usage_f) = metrics.load_percent {
                            let target_val = usage_f.round().clamp(0.0, 100.0) as u16;
                            if !device.send_temperature(target_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{} %", target_val);
                            }
                            last_displayed_val = Some(target_val);
                        } else if let Ok(mut val) = state.broadcast_value.lock() {
                            *val = "—".to_string();
                        }
                    }
                    _ => {
                        // Default: "temp"
                        if let Ok(mut lbl) = state.broadcast_label.lock() {
                            *lbl = crate::i18n::I18n::get().metric_temperature;
                        }
                        if let Some(temp_f) = metrics.temperature {
                            let target_val = calc_smoothed_temp(
                                temp_f,
                                &mut smoothed_temp,
                                last_displayed_val,
                                temp_smoothing,
                            );
                            if !device.send_temperature(target_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{} °C", target_val);
                            }
                            last_displayed_val = Some(target_val);
                        } else if let Ok(mut val) = state.broadcast_value.lock() {
                            *val = "—".to_string();
                        }
                    }
                }

                thread::sleep(Duration::from_millis(interval_ms));
            }

            // Graceful shutdown: turn off display
            debug!("Sending CMD_SHOW(0) and closing device...");
            let _ = device.send_show(false);
            device.close_device();
            state.is_connected.store(false, Ordering::SeqCst);
            info!("Monitor background service stopped.");
        });

        if let Ok(mut guard) = self.worker_handle.lock() {
            *guard = Some(handle);
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.worker_handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}
