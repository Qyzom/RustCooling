use serde::Deserialize;
use std::sync::RwLock;

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Translation {
    pub app_title: String,
    pub app_badge: String,
    pub device_name: String,

    pub device_desc_connected: String,
    pub device_desc_searching: String,
    pub status_connected: String,
    pub status_searching: String,
    pub display_card_title: String,
    pub metric_temperature: String,
    pub metric_frequency: String,
    pub metric_load: String,
    pub metric_carousel: String,
    pub chip_temp: String,
    pub chip_freq: String,
    pub chip_load: String,
    pub btn_settings: String,
    pub settings_title: String,
    pub btn_back: String,
    pub setting_display_mode: String,
    pub mode_temp: String,
    pub mode_freq: String,
    pub mode_load: String,
    pub mode_carousel: String,
    pub setting_freq_format: String,
    pub freq_ghz: String,
    pub freq_mhz: String,
    pub setting_interval: String,
    pub setting_animation: String,
    pub anim_disabled: String,
    pub anim_enabled: String,
    pub setting_language: String,
    pub setting_temp_source: String,
    pub setting_temp_smoothing: String,
    pub smoothing_off: String,
    pub temp_src_package: String,
    pub temp_src_core0: String,
    pub temp_src_avg: String,
    pub temp_src_max: String,
    pub about_title: String,
    pub about_desc: String,
    pub btn_save_return: String,
    pub setting_open_config: String,
    pub setting_debug_title: String,
    pub setting_reset_defaults: String,
    pub setting_autostart: String,
    pub autostart_off: String,
    pub autostart_on: String,
    pub tray_show: String,
    pub tray_hide: String,
    pub tray_exit: String,
    pub activation_title: String,
    pub activation_subtitle: String,
    pub activation_driver_title: String,
    pub activation_driver_desc: String,
    pub activation_driver_btn: String,
    pub activation_driver_installed: String,
    pub activation_continue_btn: String,
    pub setting_driver_title: String,
    pub driver_status_active: String,
    pub driver_status_missing: String,
    pub driver_btn_install: String,
}

static CURRENT_TRANSLATION: RwLock<Option<Translation>> = RwLock::new(None);
static CURRENT_LANG: RwLock<String> = RwLock::new(String::new());

pub struct I18n;

impl I18n {
    pub fn init(lang: &str) {
        let mut l = CURRENT_LANG.write().unwrap();
        *l = lang.to_lowercase();
        Self::load_current(&l);
    }

    pub fn set_language(lang: &str) {
        let mut l = CURRENT_LANG.write().unwrap();
        *l = lang.to_lowercase();
        Self::load_current(&l);
    }

    pub fn get() -> Translation {
        if let Ok(guard) = CURRENT_TRANSLATION.read() {
            if let Some(ref t) = *guard {
                return t.clone();
            }
        }
        Self::parse_embedded("en")
    }

    fn load_current(lang: &str) {
        let t = Self::parse_embedded(lang);
        let mut guard = CURRENT_TRANSLATION.write().unwrap();
        *guard = Some(t);
    }

    fn parse_embedded(lang: &str) -> Translation {
        let json_str = match lang {
            "ru" => include_str!("../../i18n/ru.json"),
            "zh" => include_str!("../../i18n/zh.json"),
            "de" => include_str!("../../i18n/de.json"),
            "fr" => include_str!("../../i18n/fr.json"),
            _ => include_str!("../../i18n/en.json"),
        };
        let clean_json = json_str.trim_start_matches('\u{feff}');
        serde_json::from_str(clean_json).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_translations_load_properly() {
        for lang in &["en", "ru", "zh", "de", "fr"] {
            let t = I18n::parse_embedded(lang);
            assert!(!t.app_title.is_empty(), "app_title empty for {}", lang);
            assert!(!t.device_name.is_empty(), "device_name empty for {}", lang);
            assert!(!t.about_title.is_empty(), "about_title empty for {}", lang);
            assert_eq!(t.about_title, format!("RustCooling v{}", env!("CARGO_PKG_VERSION")), "about_title mismatch for {}", lang);
            assert!(!t.setting_language.is_empty(), "setting_language empty for {}", lang);
            assert!(!t.status_connected.is_empty(), "status_connected empty for {}", lang);
        }
    }
}

