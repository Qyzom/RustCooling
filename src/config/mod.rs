use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub update_interval_ms: u64,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub high_priority: bool,
    pub language: String,
    #[serde(default = "default_display_mode")]
    pub display_mode: String, // "temp", "freq", "load", "carousel"
    #[serde(default = "default_true")]
    pub freq_in_ghz: bool, // true: 4.6 GHz (sends 46 to LCD), false: 4600 MHz
}

fn default_display_mode() -> String {
    "temp".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,
            auto_start: false,
            minimize_to_tray: true,
            start_minimized: false,
            high_priority: true,
            language: "en".to_string(),
            display_mode: "temp".to_string(),
            freq_in_ghz: true,
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("rust-cooling");
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                    return cfg;
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
