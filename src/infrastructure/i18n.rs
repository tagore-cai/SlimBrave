use fluent::{FluentArgs, FluentBundle, FluentResource};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("../../assets/i18n/en.ftl");
const ZH_FTL: &str = include_str!("../../assets/i18n/zh.ftl");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    fn langid(self) -> LanguageIdentifier {
        match self {
            Lang::En => "en".parse().expect("valid langid"),
            Lang::Zh => "zh".parse().expect("valid langid"),
        }
    }

    fn ftl(self) -> &'static str {
        match self {
            Lang::En => EN_FTL,
            Lang::Zh => ZH_FTL,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }

    pub fn all() -> [Lang; 2] {
        [Lang::En, Lang::Zh]
    }

    pub fn detect() -> Self {
        let Ok(lang) = std::env::var("LANG") else {
            return Lang::En;
        };
        let Ok(requested) = lang
            .split('.')
            .next()
            .unwrap_or("en")
            .replace('_', "-")
            .parse::<LanguageIdentifier>()
        else {
            return Lang::En;
        };
        let available: Vec<LanguageIdentifier> =
            vec![Lang::En.langid(), Lang::Zh.langid()];
        let default = Lang::En.langid();
        let negotiated = negotiate_languages(
            std::slice::from_ref(&requested),
            &available,
            Some(&default),
            NegotiationStrategy::Filtering,
        );
        if negotiated.first().copied() == Some(&Lang::Zh.langid()) {
            Lang::Zh
        } else {
            Lang::En
        }
    }
}

pub struct I18n {
    bundle: FluentBundle<FluentResource>,
    lang: Lang,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let lang = Lang::detect();
        Self {
            bundle: build_bundle(lang),
            lang,
        }
    }

    #[cfg(test)]
    pub fn with_lang(lang: Lang) -> Self {
        Self {
            bundle: build_bundle(lang),
            lang,
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        if self.lang == lang {
            return;
        }
        self.lang = lang;
        self.bundle = build_bundle(lang);
    }

    pub fn t(&self, key: &str) -> String {
        self.format_msg(key, None).unwrap_or_else(|| key.to_owned())
    }

    pub fn t_args(&self, key: &str, args: &FluentArgs<'_>) -> String {
        self.format_msg(key, Some(args))
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn option(&self, option: &str) -> String {
        let key = match option {
            "Not Set" => "option-not-set",
            "Ask" => "option-ask",
            "Block" => "option-block",
            "Allow" => "option-allow",
            "On" => "option-on",
            "Off" => "option-off",
            "Automatic" => "option-automatic",
            "Secure" => "option-secure",
            "Custom" => "option-custom",
            other => other,
        };
        self.t(key)
    }

    pub fn feature_name(&self, key: &str) -> Option<String> {
        self.format_msg(&format!("feature-{key}"), None)
    }

    pub fn feature_tooltip(&self, key: &str) -> Option<String> {
        self.format_msg(&format!("feature-tip-{key}"), None)
    }

    pub fn permission_name(&self, key: &str) -> Option<String> {
        self.format_msg(&format!("permission-{key}"), None)
    }

    pub fn permission_tooltip(&self, key: &str) -> Option<String> {
        self.format_msg(&format!("permission-tip-{key}"), None)
    }

    pub fn suggestion_text(&self, privacy: bool, security: bool) -> String {
        let key = match (privacy, security) {
            (true, true) => "suggestion-tt",
            (true, false) => "suggestion-tf",
            (false, true) => "suggestion-ft",
            (false, false) => "suggestion-ff",
        };
        self.t(key)
    }

    fn format_msg(&self, key: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
        let message = self.bundle.get_message(key)?;
        let pattern = message.value()?;
        let mut errors = Vec::new();
        let value = self.bundle.format_pattern(pattern, args, &mut errors);
        Some(value.into_owned())
    }
}

fn build_bundle(lang: Lang) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![lang.langid()]);
    let resource = FluentResource::try_new(lang.ftl().to_owned()).expect("valid ftl resource");
    bundle
        .add_resource(resource)
        .expect("ftl resource has no parse errors");
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(i18n.permission_name("DefaultGeolocationSetting"), Some("位置".to_owned()));
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
                    if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
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
}
