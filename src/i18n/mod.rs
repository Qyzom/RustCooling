use serde::Deserialize;
use std::sync::RwLock;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Translation {

    pub app_title: String,
    pub app_subtitle: String,
    pub connected: String,
    pub disconnected: String,
    pub display_title: String,
    pub display_active_metric: String,
    pub metric_temperature: String,
    pub metric_frequency: String,
    pub metric_load: String,
    pub chart_cpu_load: String,
    pub chart_cpu_freq: String,
    pub chart_realtime: String,
    pub btn_settings: String,
    pub btn_minimize: String,
    pub status_broadcasting: String,
    pub status_idle: String,
    pub unit_celsius: String,
    pub unit_mhz: String,
    pub unit_percent: String,
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

    #[allow(dead_code)]
    pub fn set_language(lang: &str) {

        let mut l = CURRENT_LANG.write().unwrap();
        *l = lang.to_lowercase();
        Self::load_current(&l);
    }

    #[allow(dead_code)]
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
            _ => include_str!("../../i18n/en.json"),
        };
        serde_json::from_str(json_str).unwrap_or_default()
    }
}
