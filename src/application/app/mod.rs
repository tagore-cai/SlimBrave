//! 应用状态与用例编排。
//!
//! - [`config`]：配置文件与持久化路径
//! - [`actions`]：用户触发的用例（应用 / 重置 / 拉取 / 导入导出 / 预设）

mod actions;
mod config;

use eframe::egui;
use fluent::fluent_args;

use crate::domain::catalog;
use crate::domain::{Browser, PlatformKind, UiState, WriteLevel};
use crate::infrastructure::i18n::{I18n, Lang};
use crate::infrastructure::platform::{self, Channel};
use crate::presentation::theme::{self, Theme, ThemeOverrides};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    Apply,
    Reset,
}

pub struct SlimBraveApp {
    pub(crate) platform: PlatformKind,
    pub(crate) browser: Browser,
    pub(crate) channels: Vec<Channel>,
    pub(crate) selected_channel: usize,
    pub(crate) state: UiState,
    pub(crate) baseline: crate::domain::StateSnapshot,
    pub(crate) status: String,
    pub(crate) report: Option<platform::ApplyReport>,
    pub(crate) dry_run: bool,
    pub(crate) pending: Option<PendingAction>,
    pub(crate) pending_message: String,
    pub(crate) i18n: I18n,
    pub(crate) theme_pref: egui::ThemePreference,
    pub(crate) theme_overrides: Option<ThemeOverrides>,
    pub(crate) last_theme: Option<egui::Theme>,
    pub(crate) user_catalog: crate::domain::catalog::UserCatalog,
    pub(crate) show_policy_manager: bool,
    pub(crate) remote: Option<crate::domain::catalog::RemoteCatalog>,
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
            baseline: crate::domain::StateSnapshot::default(),
            status: String::new(),
            report: None,
            dry_run: false,
            pending: None,
            pending_message: String::new(),
            i18n: Self::load_config()
                .and_then(|config| match config.lang.as_str() {
                    "en" => Some(I18n::with_lang(Lang::En)),
                    "zh" => Some(I18n::with_lang(Lang::Zh)),
                    _ => None,
                })
                .unwrap_or_default(),
            theme_pref,
            theme_overrides: Self::load_theme_overrides(),
            last_theme: None,
            user_catalog: Self::load_user_catalog(),
            show_policy_manager: false,
            remote: catalog::load_remote(browser),
            #[cfg(target_os = "macos")]
            profile_active: None,
            write_level: Self::load_config()
                .map(|c| {
                    if c.write_level == "machine" {
                        WriteLevel::Machine
                    } else {
                        WriteLevel::User
                    }
                })
                .unwrap_or(WriteLevel::User),
        };
        if !browser_installed {
            app.status = app.i18n.t_args(
                "browser-not-installed",
                &fluent_args!["browser" => app.i18n.t("browser-brave")],
            );
        }
        app.status = app.i18n.t("status-ready");
        match Self::load_state() {
            Some(snapshot) => {
                app.state.apply_snapshot(&snapshot, browser);
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

    pub(crate) fn request(&mut self, action: PendingAction, message: String) {
        self.pending = Some(action);
        self.pending_message = message;
    }

    pub(crate) fn run_pending(&mut self, action: PendingAction) {
        match action {
            PendingAction::Apply => self.apply(),
            PendingAction::Reset => self.reset_settings(),
        }
    }
}
