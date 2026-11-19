//! Fluent 本地化：中 / 英双语，消息查询与语言切换。
//!
//! [`lang`] 负责语言枚举、内嵌 FTL 资源与 bundle 构建；
//! 本模块提供对外翻译 API [`I18n`]。

mod lang;

#[cfg(test)]
mod tests;

use fluent::{FluentArgs, FluentBundle, FluentResource};

use lang::build_bundle;
pub use lang::Lang;

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

    pub(crate) fn with_lang(lang: Lang) -> Self {
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
