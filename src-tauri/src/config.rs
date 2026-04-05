use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub udp_port: u16,
    pub tcp_port: u16,
    pub hud_width: u32,
    pub hud_height: u32,
    pub default_text_color: String,
    pub always_on_top: bool,
    pub auto_fit_font: bool,
    pub fixed_font_size: u32,
    pub hud_x: Option<i32>,
    pub hud_y: Option<i32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            udp_port: 52100,
            tcp_port: 52101,
            hud_width: 800,
            hud_height: 300,
            default_text_color: "white".to_string(),
            always_on_top: false,
            auto_fit_font: true,
            fixed_font_size: 72,
            hud_x: None,
            hud_y: None,
        }
    }
}

pub struct AppConfig {
    pub settings: Mutex<Settings>,
    config_path: PathBuf,
}

impl AppConfig {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sndwrks-hud");
        Self::with_dir(config_dir)
    }

    pub(crate) fn with_dir(config_dir: PathBuf) -> Self {
        fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("settings.json");

        let settings = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Settings::default()
        };

        Self {
            settings: Mutex::new(settings),
            config_path,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let settings = self.settings.lock().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_values() {
        let s = Settings::default();
        assert_eq!(s.udp_port, 52100);
        assert_eq!(s.tcp_port, 52101);
        assert_eq!(s.hud_width, 800);
        assert_eq!(s.hud_height, 300);
        assert_eq!(s.default_text_color, "white");
        assert!(!s.always_on_top);
        assert!(s.auto_fit_font);
        assert_eq!(s.fixed_font_size, 72);
        assert_eq!(s.hud_x, None);
        assert_eq!(s.hud_y, None);
    }

    #[test]
    fn serialization_round_trip() {
        let s = Settings {
            udp_port: 9999,
            tcp_port: 8888,
            hud_width: 1024,
            hud_height: 768,
            default_text_color: "red".to_string(),
            always_on_top: true,
            auto_fit_font: false,
            fixed_font_size: 48,
            hud_x: Some(100),
            hud_y: Some(200),
        };
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn deserialize_missing_fields_uses_defaults() {
        let json = r#"{"udp_port": 9999}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.udp_port, 9999);
        assert_eq!(s.tcp_port, 52101); // default
        assert_eq!(s.hud_width, 800); // default
        assert_eq!(s.default_text_color, "white"); // default
    }

    #[test]
    fn deserialize_extra_fields_ignored() {
        let json = r#"{"udp_port": 52100, "unknown_field": true}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.udp_port, 52100);
    }

    #[test]
    fn app_config_save_and_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let config = AppConfig::with_dir(tmp.path().to_path_buf());

        {
            let mut s = config.settings.lock().unwrap();
            s.udp_port = 12345;
            s.default_text_color = "green".to_string();
        }
        config.save().unwrap();

        let reloaded = AppConfig::with_dir(tmp.path().to_path_buf());
        let s = reloaded.settings.lock().unwrap();
        assert_eq!(s.udp_port, 12345);
        assert_eq!(s.default_text_color, "green");
    }

    #[test]
    fn app_config_new_dir_creates_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("test-config");
        let config = AppConfig::with_dir(subdir);
        let s = config.settings.lock().unwrap();
        assert_eq!(*s, Settings::default());
    }
}
