use eframe::egui;
use fluent::fluent_args;
use serde::{Deserialize, Serialize};

use crate::domain::catalog;
use crate::domain::catalog::{RemoteCatalog, UserCatalog};
use crate::domain::{
    apply_payload_to_ui, build_apply_plan, sanitize_payload, Browser, PlatformKind, Preset,
    StateSnapshot, UiState, WriteLevel,
};
use crate::infrastructure::i18n::I18n;
use crate::infrastructure::platform::{self, ApplyReport, Channel};
use crate::presentation::theme::{self, Theme, ThemeOverrides};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    Apply,
    Reset,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AppConfig {
    theme: String,
    #[serde(default)]
    write_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_owned(),
            write_level: "user".to_owned(),
        }
    }
}

pub struct SlimBraveApp {
    pub(crate) platform: PlatformKind,
    pub(crate) browser: Browser,
    pub(crate) channels: Vec<Channel>,
    pub(crate) selected_channel: usize,
    pub(crate) state: UiState,
    pub(crate) baseline: StateSnapshot,
    pub(crate) status: String,
    pub(crate) report: Option<ApplyReport>,
    pub(crate) dry_run: bool,
    pub(crate) pending: Option<PendingAction>,
    pub(crate) pending_message: String,
    pub(crate) i18n: I18n,
    pub(crate) theme_pref: egui::ThemePreference,
    pub(crate) theme_overrides: Option<ThemeOverrides>,
    pub(crate) last_theme: Option<egui::Theme>,
    pub(crate) user_catalog: UserCatalog,
    pub(crate) show_policy_manager: bool,
    pub(crate) remote: Option<RemoteCatalog>,
    #[cfg(target_os = "macos")]
    pub(crate) profile_active: Option<bool>,
    pub(crate) write_level: WriteLevel,
}

impl SlimBraveApp {
    pub fn new(cc: &eframe::CreationContext<'_>, platform: PlatformKind) -> Self {
        theme::install_cjk_fonts(&cc.egui_ctx);

        let theme_pref = Self::load_config()
            .and_then(|config| Self::parse_theme_pref(&config.theme))
            .unwrap_or(egui::ThemePreference::Dark);

        let browser = Browser::Brave;
        let (channels, browser_installed) = platform::installed_channels(browser);
        let mut app = Self {
            platform,
            browser,
            channels,
            selected_channel: 0,
            state: UiState::default(),
            baseline: StateSnapshot::default(),
            status: String::new(),
            report: None,
            dry_run: false,
            pending: None,
            pending_message: String::new(),
            i18n: I18n::new(),
            theme_pref,
            theme_overrides: Self::load_theme_overrides(),
            last_theme: None,
            user_catalog: Self::load_user_catalog(),
            show_policy_manager: false,
            remote: catalog::load_remote(browser),
            #[cfg(target_os = "macos")]
            profile_active: None,
            write_level: Self::load_config()
                .map(|c| if c.write_level == "machine" {
                    WriteLevel::Machine
                } else {
                    WriteLevel::User
                })
                .unwrap_or(WriteLevel::User),
        };
        if !browser_installed {
            app.status = app
                .i18n
                .t_args("browser-not-installed", &fluent_args!["browser" => app.i18n.t("browser-brave")]);
        }
        app.status = app
            .i18n
            .t("status-ready");
        match Self::load_state() {
            Some(snapshot) => {
                app.state.apply_snapshot(&snapshot);
                app.update_baseline();
                app.status = app.i18n.t("status-restored");
            }
            None => app.pull_settings(),
        }
        app
    }

    pub(crate) fn current_channel(&self) -> Channel {
        self.channels[self.selected_channel]
    }

    pub(crate) fn theme(&self, egui_theme: egui::Theme) -> Theme {
        match &self.theme_overrides {
            Some(overrides) => overrides.apply(),
            None => Theme::from_egui(egui_theme),
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.state.to_snapshot() != self.baseline
    }

    pub(crate) fn update_baseline(&mut self) {
        self.baseline = self.state.to_snapshot();
    }

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

    pub(crate) fn toggle_policy(&mut self, key: &str, enabled: bool) {
        if enabled {
            self.user_catalog.remove.retain(|k| k != key);
        } else if !self.user_catalog.remove.iter().any(|k| k == key) {
            self.user_catalog.remove.push(key.to_owned());
        }
        self.save_user_catalog();
        catalog::reload();
        self.state = UiState::default();
        self.update_baseline();
        self.status = self.i18n.t(if enabled { "policy-enabled" } else { "policy-disabled" });
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn fetch_templates(&mut self) {
        self.status = self.i18n.t("fetching-templates");
        match crate::infrastructure::fetch::fetch_policies(self.browser) {
            Ok(count) => {
                self.remote = catalog::load_remote(self.browser);
                self.status = self.i18n.t_args(
                    "fetch-ok",
                    &fluent_args!["count" => count],
                );
            }
            Err(err) => {
                self.status = self
                    .i18n
                    .t_args("fetch-failed", &fluent_args!["err" => err]);
            }
        }
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

    pub(crate) fn pull_settings(&mut self) {
        let channel = self.current_channel();
        let raw = platform::merged_policy_source(&channel);
        let (payload, warnings) = sanitize_payload(&raw, self.platform);
        #[cfg(target_os = "macos")]
        let _ = &warnings;
        apply_payload_to_ui(&mut self.state, &payload, self.platform);
        self.update_baseline();

        #[cfg(target_os = "macos")]
        {
            let profile_active =
                crate::infrastructure::profile::profile_installed(&channel);
            self.profile_active = Some(profile_active);
            let has_legacy = crate::infrastructure::platform::has_legacy_plists(&channel);
            self.status = match (profile_active, has_legacy) {
                (true, true) => self.i18n.t("status-profile-legacy"),
                (true, false) => self.i18n.t("status-profile-active"),
                (false, true) => self.i18n.t("status-legacy-mode"),
                (false, false) => self.i18n.t("status-no-policies"),
            };
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.status = if warnings.is_empty() {
                self.i18n.t_args(
                    "settings-loaded",
                    &fluent_args!["channel" => channel.name.to_string()],
                )
            } else {
                self.i18n.t_args(
                    "settings-loaded-warnings",
                    &fluent_args![
                        "channel" => channel.name.to_string(),
                        "count" => warnings.len()
                    ],
                )
            };
        }
    }

    pub(crate) fn apply(&mut self) {
        let channel = self.current_channel();
        if !self.dry_run && !platform::close_brave(&channel) {
            self.status = self.i18n.t_args(
                "close-failed",
                &fluent_args!["app" => channel.app_name.to_string()],
            );
            return;
        }

        let plan = build_apply_plan(&self.state, self.platform);

        match platform::apply(&channel, &plan, self.dry_run, self.write_level) {
            Ok(report) => {
                let dry_run = report.dry_run;
                self.status = if self.platform == PlatformKind::MacOs && !dry_run {
                    if report.removed_profile == Some(true) {
                        self.i18n.t("profile-removed")
                    } else {
                        self.i18n.t("profile-generated")
                    }
                } else {
                    let key = if dry_run { "dry-run-report" } else { "applied" };
                    self.i18n.t_args(
                        key,
                        &fluent_args![
                            "writes" => report.written.len(),
                            "removals" => report.deleted.len(),
                            "channel" => report.channel.to_string(),
                            "path" => report.path.clone()
                        ],
                    )
                };
                self.update_baseline();
                self.report = Some(report);
                if !dry_run {
                    self.save_state();
                }
            }
            Err(err) => {
                self.status = self
                    .i18n
                    .t_args("apply-failed", &fluent_args!["err" => err.to_string()]);
            }
        }
    }

    pub(crate) fn reset_settings(&mut self) {
        let channel = self.current_channel();
        if !platform::close_brave(&channel) {
            self.status = self.i18n.t_args(
                "close-failed",
                &fluent_args!["app" => channel.app_name.to_string()],
            );
            return;
        }

        let removed_files = platform::remove_managed_prefs(&channel, self.write_level);
        let removed_keys = platform::strip_legacy_domain_keys(&channel);

        self.state = UiState::default();
        self.update_baseline();
        self.save_state();
        self.status = self.i18n.t_args(
            "reset-done",
            &fluent_args![
                "files" => removed_files.len(),
                "keys" => removed_keys.len()
            ],
        );
    }

    pub(crate) fn request(&mut self, action: PendingAction, message: String) {
        self.pending = Some(action);
        self.pending_message = message;
    }

    pub(crate) fn apply_preset(&mut self, preset: Preset, name_key: &str) {
        self.state.apply_preset(preset);
        self.status = self.i18n.t_args(
            "preset-loaded",
            &fluent_args!["name" => self.i18n.t(name_key)],
        );
    }

    pub(crate) fn export_settings(&mut self) {
        let snapshot = self.state.to_snapshot();
        match rfd::FileDialog::new()
            .set_file_name("SlimBraveSettings.json")
            .save_file()
        {
            Some(path) => match serde_json::to_string_pretty(&snapshot) {
                Ok(json) => match std::fs::write(&path, json) {
                    Ok(()) => {
                        self.status = self.i18n.t_args(
                            "exported",
                            &fluent_args!["path" => path.display().to_string()],
                        );
                    }
                    Err(err) => {
                        self.status = self.i18n.t_args(
                            "export-failed",
                            &fluent_args!["err" => err.to_string()],
                        );
                    }
                },
                Err(err) => {
                    self.status = self.i18n.t_args(
                        "export-failed",
                        &fluent_args!["err" => err.to_string()],
                    );
                }
            },
            None => {
                self.status = self.i18n.t("export-cancelled");
            }
        }
    }

    pub(crate) fn import_settings(&mut self) {
        match rfd::FileDialog::new().pick_file() {
            Some(path) => {
                let parsed = std::fs::read_to_string(&path)
                    .map_err(|err| err.to_string())
                    .and_then(|json| {
                        serde_json::from_str::<StateSnapshot>(&json).map_err(|err| err.to_string())
                    });
                match parsed {
                    Ok(snapshot) => {
                        self.state.apply_snapshot(&snapshot);
                        self.update_baseline();
                        self.status = self.i18n.t("imported");
                    }
                    Err(err) => {
                        self.status = self.i18n.t_args(
                            "import-failed",
                            &fluent_args!["err" => err],
                        );
                    }
                }
            }
            None => {
                self.status = self.i18n.t("import-cancelled");
            }
        }
    }

    pub(crate) fn run_pending(&mut self, action: PendingAction) {
        match action {
            PendingAction::Apply => self.apply(),
            PendingAction::Reset => self.reset_settings(),
        }
    }

    pub(crate) fn confirm_dialog(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() {
            return;
        }
        let mut confirmed = false;
        let mut cancelled = false;
        let message = self.pending_message.clone();
        egui::Window::new(self.i18n.t("confirm"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(message);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(self.i18n.t("ok")).clicked() {
                        confirmed = true;
                    }
                    if ui.button(self.i18n.t("cancel")).clicked() {
                        cancelled = true;
                    }
                });
            });
        if confirmed {
            if let Some(action) = self.pending.take() {
                self.run_pending(action);
            }
        } else if cancelled {
            self.pending = None;
            self.status = self.i18n.t("cancelled");
        }
    }
}
