use eframe::egui;
use fluent::fluent_args;

use crate::application::app::{PendingAction, SlimBraveApp};
use crate::domain::catalog;
use crate::domain::{Browser, Preset, UiState, DNS_MODE_OPTIONS, SAFE_BROWSING_OPTIONS};
use crate::infrastructure::i18n::Lang;
use crate::infrastructure::platform;
use crate::presentation::theme::{ButtonStyle, Theme};

impl SlimBraveApp {
    pub(crate) fn policy_manager_window(&mut self, ctx: &egui::Context) {
        if !self.show_policy_manager {
            return;
        }
        let keys: Vec<String> = catalog().features.iter().map(|f| f.key.clone()).collect();
        let mut open = self.show_policy_manager;
        egui::Window::new(self.i18n.t("policy-manager"))
            .open(&mut open)
            .default_size([560.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    #[cfg(not(target_os = "windows"))]
                    if ui.button(self.i18n.t("update-templates")).clicked() {
                        self.fetch_templates();
                    }
                    #[cfg(windows)]
                    {
                        ui.label(self.i18n.t("write-level-label"));
                        let mut level = self.write_level;
                        egui::ComboBox::from_id_salt("write_level")
                            .selected_text(match level {
                                crate::domain::WriteLevel::User => self.i18n.t("write-level-user"),
                                crate::domain::WriteLevel::Machine => self.i18n.t("write-level-machine"),
                            })
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut level,
                                    crate::domain::WriteLevel::User,
                                    self.i18n.t("write-level-user"),
                                );
                                ui.selectable_value(
                                    &mut level,
                                    crate::domain::WriteLevel::Machine,
                                    self.i18n.t("write-level-machine"),
                                );
                            });
                        if level != self.write_level {
                            self.write_level = level;
                            self.save_config();
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    let _ = ui;
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("policy_manager_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for key in keys {
                            let feature = catalog().feature_by_key(&key).map(|f| {
                                (
                                    f.name.clone(),
                                    f.tooltip.clone(),
                                    f.privacy,
                                    f.security,
                                )
                            });
                            let Some((name, tooltip, privacy, security)) = feature else {
                                continue;
                            };
                            let name = self.display_name(&key, &name);
                            let tooltip = self.display_tooltip(&key, &tooltip, privacy, security);
                            let mut enabled = !self.user_catalog.remove.iter().any(|k| k == &key);
                            let row = ui
                                .checkbox(&mut enabled, format!("{name}  ({key})"))
                                .on_hover_text(tooltip);
                            if row.changed() {
                                self.toggle_policy(&key, enabled);
                            }
                        }
                    });
            });
        self.show_policy_manager = open;
    }

    pub(crate) fn display_name(&self, key: &str, fallback: &str) -> String {
        if let Some(localized) = self.i18n.feature_name(key) {
            return localized;
        }
        if !fallback.is_empty() {
            return fallback.to_owned();
        }
        self.remote
            .as_ref()
            .and_then(|r| r.policies.get(key))
            .filter(|meta| !meta.name.is_empty())
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|| key.to_owned())
    }

    pub(crate) fn display_tooltip(&self, key: &str, fallback: &str, privacy: bool, security: bool) -> String {
        let body = if let Some(localized) = self.i18n.feature_tooltip(key) {
            localized
        } else if !fallback.is_empty() {
            fallback.to_owned()
        } else if let Some(meta) = self.remote.as_ref().and_then(|r| r.policies.get(key)) {
            if !meta.tooltip.is_empty() {
                meta.tooltip.clone()
            } else {
                key.to_owned()
            }
        } else {
            key.to_owned()
        };
        format!("{body}{}", self.i18n.suggestion_text(privacy, security))
    }

    fn feature_section(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        title_key: &str,
        section: &str,
    ) {
        ui.add_space(8.0);
        theme.section_title(ui, &self.i18n.t(title_key));
        ui.add_space(4.0);
        for (index, feature) in catalog().features.iter().enumerate() {
            if feature.section != section {
                continue;
            }
            let label = self.display_name(&feature.key, &feature.name);
            let tooltip = self.display_tooltip(
                &feature.key,
                &feature.tooltip,
                feature.privacy,
                feature.security,
            );
            ui.checkbox(&mut self.state.checked[index], label)
                .on_hover_text(tooltip);
        }
    }

    fn permissions_panel(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.add_space(8.0);
        theme.section_title(ui, &self.i18n.t("section-permissions"));
        ui.add_space(6.0);

        egui::Grid::new("permissions")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                for (index, permission) in catalog().permissions.iter().enumerate() {
                    let name = self
                        .i18n
                        .permission_name(&permission.key)
                        .unwrap_or_else(|| permission.name.clone());
                    let tooltip = self
                        .i18n
                        .permission_tooltip(&permission.key)
                        .unwrap_or_else(|| permission.tooltip.clone());
                    ui.label(name).on_hover_text(tooltip);
                    let selected_text = self
                        .i18n
                        .option(&permission.options[self.state.permissions[index]]);
                    egui::ComboBox::from_id_salt(&permission.key)
                        .selected_text(selected_text)
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for (option_index, option) in permission.options.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.state.permissions[index],
                                    option_index,
                                    self.i18n.option(option.as_str()),
                                );
                            }
                        });
                    ui.end_row();
                }
            });

        ui.add_space(10.0);
        theme.section_title(ui, &self.i18n.t("safe-browsing"));
        ui.add_space(4.0);
        let safe_browsing_text = self
            .state
            .safe_browsing
            .map(|index| self.i18n.option(SAFE_BROWSING_OPTIONS[index]))
            .unwrap_or_else(|| self.i18n.option("Not Set"));
        egui::ComboBox::from_id_salt("safe_browsing")
            .selected_text(safe_browsing_text)
            .width(100.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(self.state.safe_browsing.is_none(), self.i18n.option("Not Set"))
                    .clicked()
                {
                    self.state.safe_browsing = None;
                }
                for (index, option) in SAFE_BROWSING_OPTIONS.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.state.safe_browsing,
                        Some(index),
                        self.i18n.option(option),
                    );
                }
            });

        ui.add_space(10.0);
        theme.section_title(ui, &self.i18n.t("dns-over-https"));
        ui.add_space(4.0);
        egui::ComboBox::from_id_salt("dns_mode")
            .selected_text(self.i18n.option(DNS_MODE_OPTIONS[self.state.dns_mode]))
            .width(100.0)
            .show_ui(ui, |ui| {
                for (index, option) in DNS_MODE_OPTIONS.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.state.dns_mode,
                        index,
                        self.i18n.option(option),
                    );
                }
            });

        ui.add_space(4.0);
        ui.label(self.i18n.t("doh-template"));
        ui.add(egui::TextEdit::singleline(&mut self.state.dns_template).hint_text("https://dns.example/dns-query"));
    }
}

impl eframe::App for SlimBraveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(self.theme_pref);
        let egui_theme = ctx.theme();
        if self.last_theme != Some(egui_theme) {
            self.last_theme = Some(egui_theme);
            self.theme(egui_theme).apply(ctx);
        }
        let theme = self.theme(egui_theme);

        self.confirm_dialog(ctx);
        self.policy_manager_window(ctx);

        egui::TopBottomPanel::top("header")
            .frame(theme.top_bar_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("SlimBrave").color(theme.accent).strong());
                    ui.separator();
                    theme.accent_label(ui, &self.i18n.t("channel-label"));
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
                    ui.label(
                        egui::RichText::new(channels[self.selected_channel].domain)
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
                        let mut selected_browser = self.browser;
                        egui::ComboBox::from_id_salt("browser")
                            .selected_text(match selected_browser {
                                Browser::Chrome => self.i18n.t("browser-chrome"),
                                Browser::Brave => self.i18n.t("browser-brave"),
                            })
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for (browser, key) in [
                                    (Browser::Brave, "browser-brave"),
                                    (Browser::Chrome, "browser-chrome"),
                                ] {
                                    ui.selectable_value(
                                        &mut selected_browser,
                                        browser,
                                        self.i18n.t(key),
                                    );
                                }
                            });
                        if selected_browser != self.browser {
                            self.browser = selected_browser;
                            let (channels, installed) = platform::installed_channels(selected_browser);
                            self.channels = channels;
                            self.selected_channel = 0;
                            self.remote = catalog::load_remote(selected_browser);
                            self.state = UiState::default();
                            if installed {
                                self.pull_settings();
                            } else {
                                self.status = self.i18n.t_args(
                                    "browser-not-installed",
                                    &fluent_args![
                                        "browser" => self.i18n.t(if selected_browser == Browser::Brave {
                                            "browser-brave"
                                        } else {
                                            "browser-chrome"
                                        })
                                    ],
                                );
                            }
                        }
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
                        }
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
                                    ui.selectable_value(
                                        &mut selected_theme,
                                        pref,
                                        self.i18n.t(key),
                                    );
                                }
                            });
                        if selected_theme != self.theme_pref {
                            self.theme_pref = selected_theme;
                            self.last_theme = None;
                            self.save_config();
                        }
                        if self.is_dirty() {
                            ui.label(egui::RichText::new(self.i18n.t("unsaved")).color(theme.dirty).strong());
                        } else {
                            ui.label(egui::RichText::new(self.i18n.t("saved")).color(theme.saved).strong());
                        }
                    });
                });
            });

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
                        .add(theme.button(
                            &self.i18n.t("btn-pull"),
                            ButtonStyle::Warning,
                        ))
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
                        .add(theme.button(
                            &self.i18n.t("btn-reset"),
                            ButtonStyle::Danger,
                        ))
                        .clicked()
                    {
                        self.request(PendingAction::Reset, self.i18n.t("confirm-reset"));
                    }
                    ui.checkbox(
                        &mut self.dry_run,
                        self.i18n.t("dry-run"),
                    )
                    .on_hover_text(self.i18n.t("dry-run"));
                    ui.separator();
                    theme.status_label(ui, &self.status);
                });
            });

        egui::SidePanel::left("telemetry_panel")
            .exact_width(340.0)
            .frame(theme.panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.feature_section(ui, &theme, "section-telemetry", "telemetry");
                        self.feature_section(ui, &theme, "section-privacy", "privacy");
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
                        self.permissions_panel(ui, &theme);
                    });
            });

        egui::CentralPanel::default()
            .frame(theme.panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("middle_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.feature_section(ui, &theme, "section-brave", "brave");
                        self.feature_section(ui, &theme, "section-perf", "perf");
                    });
            });
    }
}

