//! 策略控件：名称 / 提示本地化、策略分组勾选、站点权限与安全设置面板。

use eframe::egui;

use crate::application::app::SlimBraveApp;
use crate::domain::catalog;
use crate::domain::{DNS_MODE_OPTIONS, SAFE_BROWSING_OPTIONS};
use crate::presentation::theme::Theme;

impl SlimBraveApp {
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

    pub(crate) fn display_tooltip(
        &self,
        key: &str,
        fallback: &str,
        privacy: bool,
        security: bool,
    ) -> String {
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

    pub(crate) fn feature_section(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        title_key: &str,
        section: &str,
    ) {
        ui.add_space(8.0);
        theme.section_title(ui, &self.i18n.t(title_key));
        ui.add_space(4.0);
        for (index, feature) in catalog().features_for(self.browser) {
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

    pub(crate) fn permissions_panel(&mut self, ui: &mut egui::Ui, theme: &Theme) {
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
                    .selectable_label(
                        self.state.safe_browsing.is_none(),
                        self.i18n.option("Not Set"),
                    )
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
                    ui.selectable_value(&mut self.state.dns_mode, index, self.i18n.option(option));
                }
            });

        ui.add_space(4.0);
        ui.label(self.i18n.t("doh-template"));
        ui.add(
            egui::TextEdit::singleline(&mut self.state.dns_template)
                .hint_text("https://dns.example/dns-query"),
        );
    }
}
