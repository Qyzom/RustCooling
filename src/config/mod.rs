use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Persistent application configuration stored in the standard XDG / OS config directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Whether the application launches automatically on system login.
    #[serde(default = "default_false")]
    pub auto_start: bool,
    /// Refresh interval for screen updates in milliseconds (100–3000 ms).
    #[serde(default = "default_interval")]
    pub update_interval_ms: u64,
    /// Active telemetry metric shown on pump screen: "temp", "load".
    #[serde(default = "default_display_mode")]
    pub display_mode: String,
    /// CPU temperature source sensor: "package", "core0", "avg", "max".
    #[serde(default = "default_temp_source")]
    pub temp_source: String,
    /// Whether transition animation is enabled (roller effect with 50 ms tick).
    #[serde(default = "default_false")]
    pub animation_enabled: bool,
    /// UI language code: "en", "ru", "zh", "de", "fr".
    #[serde(default = "default_language")]
    pub language: String,
    /// USB Vendor ID of the target display controller (default: 0x1A86).
    #[serde(default = "default_vid")]
    pub custom_vid: u16,
    /// USB Product ID of the target display controller (default: 0xE317).
    #[serde(default = "default_pid")]
    pub custom_pid: u16,
    /// Temperature smoothing threshold in degrees C (0 = Off, 1-5 = hysteresis / deadband).
    #[serde(default = "default_temp_smoothing")]
    pub temp_smoothing: u32,
    /// Whether the first-run activation / setup wizard was completed.
    #[serde(default = "default_false")]
    pub first_run_completed: bool,
}

fn default_temp_smoothing() -> u32 {
    1
}

fn default_false() -> bool {
    false
}

fn default_interval() -> u64 {
    300
}

fn default_display_mode() -> String {
    "temp".to_string()
}

fn default_temp_source() -> String {
    "package".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_vid() -> u16 {
    0x1A86
}

fn default_pid() -> u16 {
    0xE317
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            update_interval_ms: 300,
            display_mode: "temp".to_string(),
            temp_source: "package".to_string(),
            animation_enabled: false,
            language: "en".to_string(),
            custom_vid: 0x1A86,
            custom_pid: 0xE317,
            temp_smoothing: 1,
            first_run_completed: false,
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.to_path_buf();
            }
        }
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("RustCooling");
        let _ = fs::create_dir_all(&path);
        path
    }

    fn config_path() -> PathBuf {
        let mut path = Self::config_dir();
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();

        // One-time migration from legacy "rust-cooling" directory if needed
        if !path.exists() {
            if let Some(mut old_dir) = dirs::config_dir() {
                old_dir.push("rust-cooling");
                let old_file = old_dir.join("config.json");
                if old_file.exists() {
                    let _ = fs::copy(&old_file, &path);
                }
            }
        }

        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&data) {
                    if let Ok(mut cfg) = serde_json::from_value::<AppConfig>(val.clone()) {
                        if cfg.display_mode != "temp" && cfg.display_mode != "load" {
                            cfg.display_mode = "temp".to_string();
                        }
                        if cfg.update_interval_ms > 3000 || cfg.update_interval_ms < 100 {
                            cfg.update_interval_ms = cfg.update_interval_ms.clamp(100, 3000);
                        }
                        cfg.temp_smoothing = cfg.temp_smoothing.min(5);
                        return cfg;
                    }
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        Ok(())
    }
}
