use super::lang::{EN_FTL, ZH_FTL};
use super::{I18n, Lang};
use fluent::fluent_args;

#[test]
fn english_returns_keys() {
    let i18n = I18n::with_lang(Lang::En);
    assert_eq!(i18n.t("btn-apply"), "Apply Settings");
    assert_eq!(i18n.t("section-telemetry"), "Telemetry and Reporting");
}

#[test]
fn chinese_translates_ui_and_features() {
    let i18n = I18n::with_lang(Lang::Zh);
    assert_eq!(i18n.t("btn-apply"), "应用设置");
    assert_eq!(
        i18n.feature_name("MetricsReportingEnabled"),
        Some("禁用指标上报".to_owned())
    );
    assert_eq!(
        i18n.feature_tooltip("MetricsReportingEnabled"),
        Some("停止 Brave 发送匿名使用数据和崩溃报告。".to_owned())
    );
    assert_eq!(
        i18n.permission_name("DefaultGeolocationSetting"),
        Some("位置".to_owned())
    );
    assert_eq!(i18n.option("Not Set"), "未设置");
}

#[test]
fn missing_keys_fall_back_to_key() {
    let i18n = I18n::with_lang(Lang::Zh);
    assert_eq!(i18n.t("no-such-key"), "no-such-key");
    assert_eq!(i18n.feature_name("NotARealKey"), None);
}

#[test]
fn args_interpolate() {
    let i18n = I18n::with_lang(Lang::Zh);
    let rendered = i18n.t_args(
        "settings-loaded",
        &fluent_args!["channel" => "Release".to_string()],
    );
    assert!(rendered.starts_with("已从"), "got: {rendered}");
    assert!(rendered.contains("Release"), "got: {rendered}");
    assert!(rendered.ends_with("加载设置。"), "got: {rendered}");
}

#[test]
fn suggestion_text_localized() {
    let en = I18n::with_lang(Lang::En);
    assert!(en.suggestion_text(true, true).contains("Ticked"));
    let zh = I18n::with_lang(Lang::Zh);
    assert!(zh.suggestion_text(true, true).contains("勾选"));
}

#[test]
fn all_ftl_messages_resolve() {
    for lang in Lang::all() {
        let i18n = I18n::with_lang(lang);
        for key in [
            "channel-label",
            "quick-presets",
            "high-privacy",
            "high-security",
            "section-telemetry",
            "section-privacy",
            "section-brave",
            "section-perf",
            "section-permissions",
            "safe-browsing",
            "dns-over-https",
            "doh-template",
            "btn-export",
            "btn-import",
            "btn-pull",
            "btn-apply",
            "btn-reset",
            "dry-run",
            "saved",
            "unsaved",
            "confirm",
            "ok",
            "cancel",
            "option-not-set",
            "option-ask",
            "option-block",
            "option-allow",
            "option-on",
            "option-off",
            "option-automatic",
            "option-secure",
            "option-custom",
            "status-ready",
            "status-restored",
            "settings-loaded",
            "settings-loaded-warnings",
            "applied",
            "dry-run-report",
            "apply-failed",
            "close-failed",
            "reset-done",
            "cancelled",
            "exported",
            "export-failed",
            "export-cancelled",
            "imported",
            "import-failed",
            "import-cancelled",
            "state-save-failed",
            "preset-loaded",
            "confirm-close",
            "confirm-reset",
            "suggestion-tt",
            "suggestion-tf",
            "suggestion-ft",
            "suggestion-ff",
        ] {
            assert!(!i18n.t(key).is_empty(), "{lang:?}: missing message {key}");
        }
    }
}

#[test]
fn en_and_zh_ftl_have_identical_key_sets() {
    fn keys(ftl: &str) -> std::collections::BTreeSet<String> {
        let mut set = std::collections::BTreeSet::new();
        for line in ftl.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    set.insert(key.to_owned());
                }
            }
        }
        set
    }
    let en = keys(EN_FTL);
    let zh = keys(ZH_FTL);
    assert_eq!(en, zh, "en.ftl and zh.ftl key sets must match");
}

#[test]
fn every_feature_and_permission_has_translation() {
    for feature in crate::domain::catalog::init().features.iter() {
        for lang in Lang::all() {
            let i18n = I18n::with_lang(lang);
            assert!(
                i18n.feature_name(&feature.key).is_some(),
                "{lang:?}: missing feature name for {}",
                feature.key
            );
            assert!(
                i18n.feature_tooltip(&feature.key).is_some(),
                "{lang:?}: missing feature tooltip for {}",
                feature.key
            );
        }
    }
    for permission in crate::domain::catalog::init().permissions.iter() {
        for lang in Lang::all() {
            let i18n = I18n::with_lang(lang);
            assert!(
                i18n.permission_name(&permission.key).is_some(),
                "{lang:?}: missing permission name for {}",
                permission.key
            );
            assert!(
                i18n.permission_tooltip(&permission.key).is_some(),
                "{lang:?}: missing permission tooltip for {}",
                permission.key
            );
        }
    }
}

#[test]
fn pull_button_reflects_current_browser() {
    use fluent::fluent_args;
    for lang in Lang::all() {
        let i18n = I18n::with_lang(lang);
        for (browser_key, expected_name) in
            [("browser-brave", "Brave"), ("browser-chrome", "Chrome")]
        {
            let name = i18n.t(browser_key);
            let rendered = i18n.t_args("btn-pull", &fluent_args!["browser" => name.clone()]);
            assert!(
                rendered.contains(&name),
                "{lang:?}: btn-pull must include {name}"
            );
            assert!(
                !rendered.contains("{$browser}"),
                "{lang:?}: placeholder must be substituted"
            );
            let _ = expected_name;
        }
    }
}

#[test]
fn confirm_reset_reflects_current_browser() {
    use fluent::fluent_args;
    for lang in Lang::all() {
        let i18n = I18n::with_lang(lang);
        let name = i18n.t("browser-chrome");
        let rendered = i18n.t_args("confirm-reset", &fluent_args!["browser" => name.clone()]);
        assert!(
            rendered.contains(&name),
            "{lang:?}: confirm-reset must include {name}"
        );
        assert!(
            !rendered.contains("{$browser}"),
            "{lang:?}: placeholder must be substituted"
        );
    }
}
