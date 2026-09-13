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
    pub temp_history: Arc<Mutex<Vec<f32>>>,
    pub load_history: Arc<Mutex<Vec<f32>>>,
    pub freq_history: Arc<Mutex<Vec<f32>>>,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(CpuMetrics::default())),
            is_connected: Arc::new(AtomicBool::new(false)),
            broadcast_label: Arc::new(Mutex::new(crate::i18n::I18n::get().metric_temperature)),
            broadcast_value: Arc::new(Mutex::new("—".to_string())),
            temp_history: Arc::new(Mutex::new(vec![0.0; 16])),
            load_history: Arc::new(Mutex::new(vec![0.0; 16])),
            freq_history: Arc::new(Mutex::new(vec![0.0; 16])),
        }
    }
}

pub struct MonitorService {
    config: Arc<Mutex<AppConfig>>,
    state: MonitorState,
    running: Arc<AtomicBool>,
    device: Arc<DeviceManager>,
}

impl MonitorService {
    pub fn new(config: Arc<Mutex<AppConfig>>) -> Self {
        Self {
            config,
            state: MonitorState::new(),
            running: Arc::new(AtomicBool::new(false)),
            device: Arc::new(DeviceManager::new()),
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

        thread::spawn(move || {
            info!("Monitor background service started.");
            let mut telemetry = create_telemetry_provider();
            let mut was_connected = false;

            while running.load(Ordering::Relaxed) {
                // Ensure device is connected
                if !device.is_connected() {
                    if device.open_device() {
                        info!("ID-COOLING LCD Display connected.");
                        let _ = device.send_show(true);
                        state.is_connected.store(true, Ordering::SeqCst);
                        was_connected = true;
                    } else {
                        if was_connected {
                            warn!("ID-COOLING LCD Display disconnected. Retrying in 2 seconds...");
                            state.is_connected.store(false, Ordering::SeqCst);
                            was_connected = false;
                        }
                        thread::sleep(Duration::from_millis(2000));
                        continue;
                    }
                }

                // Update Telemetry
                telemetry.update();
                let metrics = telemetry.get_metrics();

                if let Ok(mut m) = state.metrics.lock() {
                    *m = metrics.clone();
                }

                // Update history buffers
                if let Some(t) = metrics.temperature {
                    if let Ok(mut hist) = state.temp_history.lock() {
                        if hist.len() >= 16 {
                            hist.remove(0);
                        }
                        hist.push(t);
                    }
                }
                if let Some(l) = metrics.load_percent {
                    if let Ok(mut hist) = state.load_history.lock() {
                        if hist.len() >= 16 {
                            hist.remove(0);
                        }
                        hist.push(l);
                    }
                }
                if let Some(f) = metrics.frequency_mhz {
                    if let Ok(mut hist) = state.freq_history.lock() {
                        if hist.len() >= 16 {
                            hist.remove(0);
                        }
                        hist.push(f);
                    }
                }

                // Read current display configuration
                let (display_mode, freq_in_ghz, interval_ms) = {
                    let cfg = config.lock().unwrap();
                    (cfg.display_mode.clone(), cfg.freq_in_ghz, cfg.update_interval_ms.max(300))
                };

                match display_mode.as_str() {
                    "freq" => {
                        if let Some(freq_f) = metrics.frequency_mhz {
                            let send_val = if freq_in_ghz {
                                ((freq_f / 100.0).round() as u16).min(99)
                            } else {
                                freq_f.round() as u16
                            };
                            if !device.send_frequency(send_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut lbl) = state.broadcast_label.lock() {
                                *lbl = crate::i18n::I18n::get().metric_frequency;
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = if freq_in_ghz {
                                    format!("{:.1} GHz", freq_f / 1000.0)
                                } else {
                                    format!("{} MHz", freq_f.round())
                                };
                            }
                        }
                    }
                    "load" => {
                        if let Some(usage_f) = metrics.load_percent {
                            let usage_val = usage_f.round() as u16;
                            if !device.send_usage(usage_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut lbl) = state.broadcast_label.lock() {
                                *lbl = crate::i18n::I18n::get().metric_load;
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{} %", usage_val);
                            }
                        }
                    }
                    "carousel" => {
                        // Staggered sending of all 3 frames
                        if let Some(temp_f) = metrics.temperature {
                            let temp_val = temp_f.round() as u16;
                            let _ = device.send_temperature(temp_val);
                            if let Ok(mut lbl) = state.broadcast_label.lock() {
                                *lbl = crate::i18n::I18n::get().metric_carousel;
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{} °C", temp_val);
                            }
                        }
                        thread::sleep(Duration::from_millis(100));

                        if let Some(freq_f) = metrics.frequency_mhz {
                            let send_val = if freq_in_ghz {
                                ((freq_f / 100.0).round() as u16).min(99)
                            } else {
                                freq_f.round() as u16
                            };
                            let _ = device.send_frequency(send_val);
                        }
                        thread::sleep(Duration::from_millis(100));

                        if let Some(usage_f) = metrics.load_percent {
                            let usage_val = usage_f.round() as u16;
                            let _ = device.send_usage(usage_val);
                        }
                    }
                    _ => {
                        // Default: "temp"
                        if let Some(temp_f) = metrics.temperature {
                            let temp_val = temp_f.round() as u16;
                            if !device.send_temperature(temp_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut lbl) = state.broadcast_label.lock() {
                                *lbl = crate::i18n::I18n::get().metric_temperature;
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{} °C", temp_val);
                            }
                        }
                    }
                }

                // Sleep the remainder of the configured update interval
                let elapsed_ms = if display_mode == "carousel" { 200 } else { 0 };
                let remaining_ms = interval_ms.saturating_sub(elapsed_ms);
                thread::sleep(Duration::from_millis(remaining_ms));
            }


            // Graceful shutdown: turn off display
            debug!("Sending CMD_SHOW(0) and closing device...");
            let _ = device.send_show(false);
            device.close_device();
            state.is_connected.store(false, Ordering::SeqCst);
            info!("Monitor background service stopped.");
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
