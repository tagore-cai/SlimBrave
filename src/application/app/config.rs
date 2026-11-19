//! 应用配置与状态持久化。

use eframe::egui;
use fluent::fluent_args;
use serde::{Deserialize, Serialize};

use crate::domain::catalog::UserCatalog;
use crate::domain::{StateSnapshot, WriteLevel};
use crate::infrastructure::i18n::Lang;
use crate::presentation::theme::ThemeOverrides;

use super::SlimBraveApp;

#[derive(Serialize, Deserialize)]
pub(crate) struct AppConfig {
    pub(crate) theme: String,
    #[serde(default)]
    pub(crate) write_level: String,
    #[serde(default)]
    pub(crate) lang: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_owned(),
            write_level: "user".to_owned(),
            lang: String::new(),
        }
    }
}

impl SlimBraveApp {
    pub(crate) fn state_file_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".config/slimbrave/SlimBraveState.json"))
    }

    pub(crate) fn config_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".config/slimbrave/config.json"))
    }

    pub(crate) fn theme_file_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".config/slimbrave/theme.json"))
    }

    pub(crate) fn user_catalog_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".config/slimbrave/catalog.json"))
    }

    pub(crate) fn load_user_catalog() -> UserCatalog {
        let Some(path) = Self::user_catalog_path() else {
            return UserCatalog::default();
        };
        let Ok(json) = std::fs::read_to_string(path) else {
            return UserCatalog::default();
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub(crate) fn save_user_catalog(&self) {
        let Some(path) = Self::user_catalog_path() else {
            return;
        };
        let empty = self.user_catalog.features.is_empty()
            && self.user_catalog.permissions.is_empty()
            && self.user_catalog.remove.is_empty()
            && self.user_catalog.presets.privacy.is_empty()
            && self.user_catalog.presets.security.is_empty();
        if empty {
            let _ = std::fs::remove_file(&path);
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(&self.user_catalog) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, json);
    }

    pub(crate) fn parse_theme_pref(value: &str) -> Option<egui::ThemePreference> {
        match value {
            "system" => Some(egui::ThemePreference::System),
            "dark" => Some(egui::ThemePreference::Dark),
            "light" => Some(egui::ThemePreference::Light),
            _ => None,
        }
    }

    pub(crate) fn theme_pref_key(pref: egui::ThemePreference) -> &'static str {
        match pref {
            egui::ThemePreference::System => "system",
            egui::ThemePreference::Dark => "dark",
            egui::ThemePreference::Light => "light",
        }
    }

    pub(crate) fn save_config(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        let config = AppConfig {
            theme: Self::theme_pref_key(self.theme_pref).to_owned(),
            write_level: match self.write_level {
                WriteLevel::User => "user".to_owned(),
                WriteLevel::Machine => "machine".to_owned(),
            },
            lang: match self.i18n.lang() {
                Lang::En => "en".to_owned(),
                Lang::Zh => "zh".to_owned(),
            },
        };
        let Ok(json) = serde_json::to_string_pretty(&config) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, json);
    }

    pub(crate) fn load_config() -> Option<AppConfig> {
        let path = Self::config_path()?;
        let json = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<AppConfig>(&json).ok()
    }

    pub(crate) fn load_theme_overrides() -> Option<ThemeOverrides> {
        let path = Self::theme_file_path()?;
        let json = std::fs::read_to_string(path).ok()?;
        ThemeOverrides::from_json(&json).ok()
    }

    pub(crate) fn save_state(&mut self) {
        let Some(path) = Self::state_file_path() else {
            return;
        };
        let Ok(json) = serde_json::to_string_pretty(&self.state.to_snapshot()) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(err) = std::fs::write(&path, json) {
            self.status = self
                .i18n
                .t_args("state-save-failed", &fluent_args!["err" => err.to_string()]);
        }
    }

    pub(crate) fn load_state() -> Option<StateSnapshot> {
        let path = Self::state_file_path()?;
        let json = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<StateSnapshot>(&json).ok()
    }
}
