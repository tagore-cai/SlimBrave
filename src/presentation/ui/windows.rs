//! 策略管理窗口：逐条启用 / 禁用策略，更新官方模板，切换写入级别。

use eframe::egui;

use crate::application::app::SlimBraveApp;
use crate::domain::catalog;

impl SlimBraveApp {
    pub(crate) fn policy_manager_window(&mut self, ctx: &egui::Context) {
        if !self.show_policy_manager {
            return;
        }
        let keys: Vec<String> = catalog()
            .features_for(self.browser)
            .map(|(_, f)| f.key.clone())
            .collect();
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
                                crate::domain::WriteLevel::Machine => {
                                    self.i18n.t("write-level-machine")
                                }
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
                                (f.name.clone(), f.tooltip.clone(), f.privacy, f.security)
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
}
