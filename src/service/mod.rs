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
            info!("Monitor background service started.");
            let mut telemetry = create_telemetry_provider();
            let mut was_connected = false;
            let mut active_vid = 0u16;
            let mut active_pid = 0u16;
            let mut last_displayed_val: Option<u16> = None;
            let mut last_display_mode = String::new();

            while running.load(Ordering::Relaxed) {
                // Read current display configuration
                let (
                    display_mode,
                    interval_ms,
                    temp_source,
                    animation_type,
                    custom_vid,
                    custom_pid,
                ) = {
                    let cfg = config.lock().unwrap();
                    (
                        cfg.display_mode.clone(),
                        cfg.update_interval_ms.clamp(100, 3000),
                        cfg.temp_source.clone(),
                        cfg.animation_type.clone(),
                        cfg.custom_vid,
                        cfg.custom_pid,
                    )
                };

                // Reset animation state if mode changed
                if display_mode != last_display_mode {
                    last_displayed_val = None;
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

                match display_mode.as_str() {
                    "freq" => {
                        if let Some(freq_f) = metrics.frequency_mhz {
                            let send_val = ((freq_f / 100.0).round() as u16).min(99);
                            if !device.send_frequency(send_val) {
                                state.is_connected.store(false, Ordering::SeqCst);
                            }
                            if let Ok(mut lbl) = state.broadcast_label.lock() {
                                *lbl = crate::i18n::I18n::get().metric_frequency;
                            }
                            if let Ok(mut val) = state.broadcast_value.lock() {
                                *val = format!("{:.1} GHz", freq_f / 1000.0);
                            }
                        }
                        thread::sleep(Duration::from_millis(interval_ms));
                    }
                    "load" => {
                        if let Ok(mut lbl) = state.broadcast_label.lock() {
                            *lbl = crate::i18n::I18n::get().metric_load;
                        }
                        if let Some(usage_f) = metrics.load_percent {
                            let target_val = usage_f.round().clamp(0.0, 100.0) as u16;
                            match last_displayed_val {
                                None => {
                                    let ok_t = device.send_temperature(target_val);
                                    let ok_u = device.send_usage(target_val);
                                    if !ok_t || !ok_u {
                                        state.is_connected.store(false, Ordering::SeqCst);
                                    }
                                    if let Ok(mut val) = state.broadcast_value.lock() {
                                        *val = format!("{} %", target_val);
                                    }
                                    last_displayed_val = Some(target_val);
                                    thread::sleep(Duration::from_millis(interval_ms));
                                }
                                Some(prev_val) => {
                                    if prev_val == target_val || animation_type == "direct" {
                                        let ok_t = device.send_temperature(target_val);
                                        let ok_u = device.send_usage(target_val);
                                        if !ok_t || !ok_u {
                                            state.is_connected.store(false, Ordering::SeqCst);
                                        }
                                        if let Ok(mut val) = state.broadcast_value.lock() {
                                            *val = format!("{} %", target_val);
                                        }
                                        last_displayed_val = Some(target_val);
                                        thread::sleep(Duration::from_millis(interval_ms));
                                    } else {
                                        let diff = target_val as i32 - prev_val as i32;
                                        let steps = diff.unsigned_abs() as usize;
                                        let step_dir = if diff > 0 { 1 } else { -1 };

                                        let step_delay_ms = calculate_step_delay(
                                            &animation_type,
                                            steps,
                                            interval_ms,
                                        );

                                        let mut curr = prev_val as i32;
                                        for _ in 0..steps {
                                            if !running.load(Ordering::Relaxed) {
                                                break;
                                            }
                                            curr += step_dir;
                                            let curr_u16 = curr.clamp(0, 100) as u16;
                                            let ok_t = device.send_temperature(curr_u16);
                                            let ok_u = device.send_usage(curr_u16);
                                            if !ok_t || !ok_u {
                                                state.is_connected.store(false, Ordering::SeqCst);
                                                break;
                                            }
                                            if let Ok(mut val) = state.broadcast_value.lock() {
                                                *val = format!("{} %", curr_u16);
                                            }
                                            thread::sleep(Duration::from_millis(step_delay_ms));
                                        }

                                        last_displayed_val = Some(target_val);
                                        let time_spent = step_delay_ms * steps as u64;
                                        if time_spent < interval_ms {
                                            thread::sleep(Duration::from_millis(
                                                interval_ms - time_spent,
                                            ));
                                        }
                                    }
                                }
                            }
                        } else {
                            thread::sleep(Duration::from_millis(interval_ms));
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
                            let send_val = ((freq_f / 100.0).round() as u16).min(99);
                            let _ = device.send_frequency(send_val);
                        }
                        thread::sleep(Duration::from_millis(100));

                        if let Some(usage_f) = metrics.load_percent {
                            let usage_val = usage_f.round() as u16;
                            let _ = device.send_usage(usage_val);
                        }
                        let remaining_ms = interval_ms.saturating_sub(200);
                        thread::sleep(Duration::from_millis(remaining_ms));
                    }
                    _ => {
                        // Default: "temp"
                        if let Ok(mut lbl) = state.broadcast_label.lock() {
                            *lbl = crate::i18n::I18n::get().metric_temperature;
                        }
                        if let Some(temp_f) = metrics.temperature {
                            let target_val = temp_f.round().clamp(0.0, 199.0) as u16;
                            match last_displayed_val {
                                None => {
                                    if !device.send_temperature(target_val) {
                                        state.is_connected.store(false, Ordering::SeqCst);
                                    }
                                    if let Ok(mut val) = state.broadcast_value.lock() {
                                        *val = format!("{} °C", target_val);
                                    }
                                    last_displayed_val = Some(target_val);
                                    thread::sleep(Duration::from_millis(interval_ms));
                                }
                                Some(prev_val) => {
                                    if prev_val == target_val || animation_type == "direct" {
                                        if !device.send_temperature(target_val) {
                                            state.is_connected.store(false, Ordering::SeqCst);
                                        }
                                        if let Ok(mut val) = state.broadcast_value.lock() {
                                            *val = format!("{} °C", target_val);
                                        }
                                        last_displayed_val = Some(target_val);
                                        thread::sleep(Duration::from_millis(interval_ms));
                                    } else {
                                        let diff = target_val as i32 - prev_val as i32;
                                        let steps = diff.unsigned_abs() as usize;
                                        let step_dir = if diff > 0 { 1 } else { -1 };

                                        let step_delay_ms = calculate_step_delay(
                                            &animation_type,
                                            steps,
                                            interval_ms,
                                        );

                                        let mut curr = prev_val as i32;
                                        for _ in 0..steps {
                                            if !running.load(Ordering::Relaxed) {
                                                break;
                                            }
                                            curr += step_dir;
                                            let curr_u16 = curr.clamp(0, 199) as u16;
                                            if !device.send_temperature(curr_u16) {
                                                state.is_connected.store(false, Ordering::SeqCst);
                                                break;
                                            }
                                            if let Ok(mut val) = state.broadcast_value.lock() {
                                                *val = format!("{} °C", curr_u16);
                                            }
                                            thread::sleep(Duration::from_millis(step_delay_ms));
                                        }

                                        last_displayed_val = Some(target_val);
                                        let time_spent = step_delay_ms * steps as u64;
                                        if time_spent < interval_ms {
                                            thread::sleep(Duration::from_millis(
                                                interval_ms - time_spent,
                                            ));
                                        }
                                    }
                                }
                            }
                        } else {
                            thread::sleep(Duration::from_millis(interval_ms));
                        }
                    }
                }
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

pub fn calculate_step_delay(animation_type: &str, steps: usize, interval_ms: u64) -> u64 {
    if steps == 0 {
        return interval_ms;
    }
    if animation_type == "roller" {
        let base_delay = 10u64;
        if (steps as u64 * base_delay) > interval_ms {
            ((interval_ms as f64) / (steps as f64)).floor().max(1.0) as u64
        } else {
            base_delay
        }
    } else {
        // smooth
        ((interval_ms as f64) / (steps as f64)).floor().max(1.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_step_delay_smooth() {
        // 1000ms / 10 steps = 100ms
        assert_eq!(calculate_step_delay("smooth", 10, 1000), 100);
        // 300ms / 5 steps = 60ms
        assert_eq!(calculate_step_delay("smooth", 5, 300), 60);
        // large steps
        assert_eq!(calculate_step_delay("smooth", 100, 100), 1);
    }

    #[test]
    fn test_calculate_step_delay_roller() {
        // 10 steps * 10ms = 100ms <= 1000ms -> fixed 10ms base delay
        assert_eq!(calculate_step_delay("roller", 10, 1000), 10);
        // 100 steps * 10ms = 1000ms > 300ms -> accelerates to 300ms / 100 = 3ms
        assert_eq!(calculate_step_delay("roller", 100, 300), 3);
    }
}
