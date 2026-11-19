//! 用户触发的用例：拉取、应用、重置、预设、导入导出、模板更新、确认对话框。

use eframe::egui;
use fluent::fluent_args;

use crate::domain::catalog;
use crate::domain::{
    apply_payload_to_ui, build_apply_plan, sanitize_payload, PlatformKind, Preset,
};
use crate::infrastructure::platform;

use super::SlimBraveApp;

impl SlimBraveApp {
    pub(crate) fn toggle_policy(&mut self, key: &str, enabled: bool) {
        if enabled {
            self.user_catalog.remove.retain(|k| k != key);
        } else if !self.user_catalog.remove.iter().any(|k| k == key) {
            self.user_catalog.remove.push(key.to_owned());
        }
        self.save_user_catalog();
        catalog::reload();
        self.state = crate::domain::UiState::default();
        self.update_baseline();
        self.status = self.i18n.t(if enabled {
            "policy-enabled"
        } else {
            "policy-disabled"
        });
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn fetch_templates(&mut self) {
        self.status = self.i18n.t("fetching-templates");
        match crate::infrastructure::fetch::fetch_policies(self.browser) {
            Ok(count) => {
                self.remote = catalog::load_remote(self.browser);
                self.status = self
                    .i18n
                    .t_args("fetch-ok", &fluent_args!["count" => count]);
            }
            Err(err) => {
                self.status = self
                    .i18n
                    .t_args("fetch-failed", &fluent_args!["err" => err]);
            }
        }
    }

    /// UTC 时钟摘要（HH:MM:SS），仅作为"刚刚发生"的可见标记，避免引入 chrono。
    fn clock_hms() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!(
            "{:02}:{:02}:{:02}",
            (secs / 3600) % 24,
            (secs / 60) % 60,
            secs % 60
        )
    }

    pub(crate) fn pull_settings(&mut self) {
        let channel = self.current_channel();
        let raw = platform::merged_policy_source(&channel);
        let (payload, warnings) = sanitize_payload(&raw, self.platform, self.browser);
        let count = payload.len();
        apply_payload_to_ui(&mut self.state, &payload, self.platform, self.browser);
        self.update_baseline();

        #[cfg(target_os = "macos")]
        let profile_detail = {
            let profile_active = crate::infrastructure::profile::profile_installed(&channel);
            self.profile_active = Some(profile_active);
            let has_legacy = crate::infrastructure::platform::has_legacy_plists(&channel);
            self.i18n.t(match (profile_active, has_legacy) {
                (true, true) => "status-profile-legacy",
                (true, false) => "status-profile-active",
                (false, true) => "status-legacy-mode",
                (false, false) => "status-no-policies",
            })
        };

        self.pull_seq = self.pull_seq.wrapping_add(1);
        let mut receipt = self.i18n.t_args(
            "pull-receipt",
            &fluent_args![
                "count" => count,
                "channel" => channel.name.to_string(),
                "time" => Self::clock_hms(),
                "seq" => self.pull_seq.to_string()
            ],
        );
        #[cfg(target_os = "macos")]
        {
            receipt.push(' ');
            receipt.push_str(&profile_detail);
        }
        if !warnings.is_empty() {
            receipt.push(' ');
            receipt.push_str(&self.i18n.t_args(
                "pull-warnings-suffix",
                &fluent_args!["count" => warnings.len()],
            ));
        }
        self.status = receipt;
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

        let plan = build_apply_plan(&self.state, self.platform, self.browser);

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

        self.state = crate::domain::UiState::default();
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

    pub(crate) fn apply_preset(&mut self, preset: Preset, name_key: &str) {
        self.state.apply_preset(preset, self.browser);
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
                        self.status = self
                            .i18n
                            .t_args("export-failed", &fluent_args!["err" => err.to_string()]);
                    }
                },
                Err(err) => {
                    self.status = self
                        .i18n
                        .t_args("export-failed", &fluent_args!["err" => err.to_string()]);
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
                        serde_json::from_str::<crate::domain::StateSnapshot>(&json)
                            .map_err(|err| err.to_string())
                    });
                match parsed {
                    Ok(snapshot) => {
                        self.state.apply_snapshot(&snapshot, self.browser);
                        self.update_baseline();
                        self.status = self.i18n.t("imported");
                    }
                    Err(err) => {
                        self.status = self
                            .i18n
                            .t_args("import-failed", &fluent_args!["err" => err]);
                    }
                }
            }
            None => {
                self.status = self.i18n.t("import-cancelled");
            }
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
