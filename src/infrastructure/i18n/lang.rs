//! 语言枚举、内嵌 FTL 资源与 Fluent bundle 构建。

use fluent::{FluentBundle, FluentResource};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
use unic_langid::LanguageIdentifier;

pub(super) const EN_FTL: &str = include_str!("../../../assets/i18n/en.ftl");
pub(super) const ZH_FTL: &str = include_str!("../../../assets/i18n/zh.ftl");

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
        let available: Vec<LanguageIdentifier> = vec![Lang::En.langid(), Lang::Zh.langid()];
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

pub(super) fn build_bundle(lang: Lang) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![lang.langid()]);
    let resource = FluentResource::try_new(lang.ftl().to_owned()).expect("valid ftl resource");
    bundle
        .add_resource(resource)
        .expect("ftl resource has no parse errors");
    bundle
}
