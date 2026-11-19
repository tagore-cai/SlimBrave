//! 主界面各面板的渲染助手：顶栏、底栏状态、左右中内容区。

use eframe::egui;
use fluent::fluent_args;

use crate::application::app::{PendingAction, SlimBraveApp};
use crate::domain::{Browser, Preset, UiState};
use crate::infrastructure::i18n::Lang;
use crate::infrastructure::platform;
use crate::presentation::theme::{ButtonStyle, Theme};

impl SlimBraveApp {
    /// 顶栏：渠道、快捷预设、浏览器/语言/主题切换、保存状态。
    pub(super) fn header_panel(&mut self, ctx: &egui::Context, theme: &Theme) {
        egui::TopBottomPanel::top("header")
            .frame(theme.top_bar_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("SlimBrave")
                            .color(theme.accent)
                            .strong(),
                    );
                    ui.separator();
                    theme.accent_label(ui, &self.i18n.t("channel-label"));
                    self.channel_picker(ui);
                    ui.label(
                        egui::RichText::new(self.channels[self.selected_channel].domain)
                            .color(theme.muted_text),
                    );

                    ui.separator();
                    theme.accent_label(ui, &self.i18n.t("quick-presets"));
                    if ui
                        .add(theme.button(&self.i18n.t("high-privacy"), ButtonStyle::Preset))
                        .clicked()
                    {
                        self.apply_preset(Preset::Privacy, "high-privacy");
                    }
                    if ui
                        .add(theme.button(&self.i18n.t("high-security"), ButtonStyle::Preset))
                        .clicked()
                    {
                        self.apply_preset(Preset::Security, "high-security");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.i18n.t("policy-manager")).clicked() {
                            self.show_policy_manager = true;
                        }
                        self.browser_picker(ui);
                        self.language_picker(ui);
                        self.theme_picker(ui);
                        if self.is_dirty() {
                            ui.label(
                                egui::RichText::new(self.i18n.t("unsaved"))
                                    .color(theme.dirty)
                                    .strong(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(self.i18n.t("saved"))
                                    .color(theme.saved)
                                    .strong(),
                            );
                        }
                    });
                });
            });
    }

    /// 渠道下拉框：切换后重新拉取该渠道的设置。
    fn channel_picker(&mut self, ui: &mut egui::Ui) {
        let channels = self.channels.clone();
        let mut selected = self.selected_channel;
        let mut switch = false;
        egui::ComboBox::from_id_salt("channel")
            .selected_text(channels[self.selected_channel].name)
            .width(100.0)
            .show_ui(ui, |ui| {
                for (index, channel) in channels.iter().enumerate() {
                    if ui
                        .selectable_value(&mut selected, index, channel.name)
                        .clicked()
                        && index != self.selected_channel
                    {
                        switch = true;
                    }
                }
            });
        if switch && selected != self.selected_channel {
            self.selected_channel = selected;
            self.pull_settings();
        }
    }

    /// 浏览器下拉框：切换后重建渠道列表与目录缓存。
    fn browser_picker(&mut self, ui: &mut egui::Ui) {
        let mut selected_browser = self.browser;
        egui::ComboBox::from_id_salt("browser")
            .selected_text(self.i18n.t(selected_browser.i18n_key()))
            .width(90.0)
            .show_ui(ui, |ui| {
                for browser in [Browser::Brave, Browser::Chrome, Browser::Edge] {
                    ui.selectable_value(
                        &mut selected_browser,
                        browser,
                        self.i18n.t(browser.i18n_key()),
                    );
                }
            });
        if selected_browser != self.browser {
            self.browser = selected_browser;
            let (channels, installed) = platform::installed_channels(selected_browser);
            self.channels = channels;
            self.selected_channel = 0;
            self.remote = crate::domain::catalog::load_remote(selected_browser);
            self.state = UiState::default();
            if installed {
                self.pull_settings();
            } else {
                self.status = self.i18n.t_args(
                    "browser-not-installed",
                    &fluent_args![
                        "browser" => self.i18n.t(selected_browser.i18n_key())
                    ],
                );
            }
        }
    }

    /// 语言下拉框：切换后持久化偏好。
    fn language_picker(&mut self, ui: &mut egui::Ui) {
        let mut selected_lang = self.i18n.lang();
        egui::ComboBox::from_id_salt("language")
            .selected_text(selected_lang.label())
            .width(80.0)
            .show_ui(ui, |ui| {
                for lang in Lang::all() {
                    ui.selectable_value(&mut selected_lang, lang, lang.label());
                }
            });
        if selected_lang != self.i18n.lang() {
            self.i18n.set_lang(selected_lang);
            self.save_config();
        }
    }

    /// 主题下拉框：切换后重置视觉缓存并持久化偏好。
    fn theme_picker(&mut self, ui: &mut egui::Ui) {
        let mut selected_theme = self.theme_pref;
        egui::ComboBox::from_id_salt("theme")
            .selected_text(match selected_theme {
                egui::ThemePreference::System => self.i18n.t("theme-system"),
                egui::ThemePreference::Dark => self.i18n.t("theme-dark"),
                egui::ThemePreference::Light => self.i18n.t("theme-light"),
            })
            .width(80.0)
            .show_ui(ui, |ui| {
                for (pref, key) in [
                    (egui::ThemePreference::System, "theme-system"),
                    (egui::ThemePreference::Dark, "theme-dark"),
                    (egui::ThemePreference::Light, "theme-light"),
                ] {
                    ui.selectable_value(&mut selected_theme, pref, self.i18n.t(key));
                }
            });
        if selected_theme != self.theme_pref {
            self.theme_pref = selected_theme;
            self.last_theme = None;
            self.save_config();
        }
    }

    /// 底栏：导入 / 导出 / 拉取 / 应用 / 重置与状态文本。
    pub(super) fn status_panel(&mut self, ctx: &egui::Context, theme: &Theme) {
        egui::TopBottomPanel::bottom("status")
            .frame(theme.status_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(theme.button(&self.i18n.t("btn-export"), ButtonStyle::Info))
                        .clicked()
                    {
                        self.export_settings();
                    }
                    if ui
                        .add(theme.button(&self.i18n.t("btn-import"), ButtonStyle::Primary))
                        .clicked()
                    {
                        self.import_settings();
                    }
                    if ui
                        .add(
                            theme.button(
                                &self.i18n.t_args(
                                    "btn-pull",
                                    &fluent_args!["browser" => self.i18n.t(self.browser.i18n_key())],
                                ),
                                ButtonStyle::Warning,
                            ),
                        )
                        .clicked()
                    {
                        self.pull_settings();
                    }
                    ui.separator();
                    if ui
                        .add(theme.button(&self.i18n.t("btn-apply"), ButtonStyle::Success))
                        .clicked()
                    {
                        if self.dry_run {
                            self.apply();
                        } else if platform::brave_is_running(&self.current_channel()) {
                            self.request(
                                PendingAction::Apply,
                                self.i18n.t_args(
                                    "confirm-close",
                                    &fluent_args!["app" => self.current_channel().app_name.to_string()],
                                ),
                            );
                        } else {
                            self.apply();
                        }
                    }
                    if ui
                        .add(theme.button(&self.i18n.t("btn-reset"), ButtonStyle::Danger))
                        .clicked()
                    {
                        self.request(
                            PendingAction::Reset,
                            self.i18n.t_args(
                                "confirm-reset",
                                &fluent_args!["browser" => self.i18n.t(self.browser.i18n_key())],
                            ),
                        );
                    }
                    ui.checkbox(&mut self.dry_run, self.i18n.t("dry-run"))
                        .on_hover_text(self.i18n.t("dry-run"));
                });
                ui.separator();
                theme.status_label(ui, &self.status);
            });
    }

    /// 左右中三个内容面板。
    pub(super) fn content_panels(&mut self, ctx: &egui::Context, theme: &Theme) {
        egui::SidePanel::left("telemetry_panel")
            .exact_width(340.0)
            .frame(theme.panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.feature_section(ui, theme, "section-telemetry", "telemetry");
                        self.feature_section(ui, theme, "section-privacy", "privacy");
                    });
            });

        egui::SidePanel::right("brave_panel")
            .exact_width(380.0)
            .frame(theme.panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("right_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.permissions_panel(ui, theme);
                    });
            });

        egui::CentralPanel::default()
            .frame(theme.panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("middle_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.feature_section(ui, theme, "section-brave", "brave");
                        self.feature_section(ui, theme, "section-perf", "perf");
                        self.feature_section(ui, theme, "section-edge", "edge");
                    });
            });
    }
}
