pub mod catalog;
pub mod payload;
pub mod state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteLevel {
    User,
    Machine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    MacOs,
    Windows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Brave,
}

/// 仅 Brave 支持的策略键。
///
/// 依据：`Brave*` 前缀为 Brave 自有策略；`IPFSEnabled`（Brave 内置 IPFS）与
/// `TorDisabled`（Brave 内置 Tor 窗口）是 Brave 特有功能，均不在 Chromium
/// 的 `components/policy/resources/templates/policy_definitions` 标准定义中。
/// Chrome 不应显示或写入这些键。
pub const BRAVE_ONLY_KEYS: &[&str] = &[
    "BraveP3AEnabled",
    "BraveStatsPingEnabled",
    "BraveWebDiscoveryEnabled",
    "BraveDeAMPEnabled",
    "BraveDebouncingEnabled",
    "BraveTrackersStrippingEnabled",
    "BraveRewardsDisabled",
    "BraveWalletDisabled",
    "BraveVPNDisabled",
    "BraveAIChatEnabled",
    "BraveNewsDisabled",
    "BraveTalkDisabled",
    "BraveSpeedreaderEnabled",
    "BraveWaybackMachineEnabled",
    "BravePlaylistEnabled",
    "IPFSEnabled",
    "TorDisabled",
];

impl Browser {
    pub fn cache_file(self) -> &'static str {
        match self {
            Browser::Chrome => "catalog.remote-chrome.json",
            Browser::Brave => "catalog.remote-brave.json",
        }
    }

    /// Fluent 消息键，用于把浏览器名插入动态文案。
    pub fn i18n_key(self) -> &'static str {
        match self {
            Browser::Chrome => "browser-chrome",
            Browser::Brave => "browser-brave",
        }
    }

    /// 该策略键是否适用于本浏览器。Brave 支持全部策略，Chrome 不含 Brave 专属项。
    pub fn supports_policy(self, key: &str) -> bool {
        match self {
            Browser::Brave => true,
            Browser::Chrome => !BRAVE_ONLY_KEYS.contains(&key),
        }
    }
}

pub use catalog::init as catalog;
pub use payload::{
    apply_payload_to_ui, build_apply_plan, sanitize_payload, ApplyPlan, RawValue, WriteValue,
};
pub use state::{Preset, StateSnapshot, UiState, DNS_MODE_OPTIONS, SAFE_BROWSING_OPTIONS};

#[cfg(test)]
mod browser_policy_tests {
    use super::*;

    #[test]
    fn chrome_rejects_brave_only_policies() {
        for key in BRAVE_ONLY_KEYS {
            assert!(
                !Browser::Chrome.supports_policy(key),
                "{key} must be hidden in Chrome"
            );
            assert!(
                Browser::Brave.supports_policy(key),
                "{key} must be shown in Brave"
            );
        }
    }

    #[test]
    fn chrome_accepts_shared_policies() {
        for key in [
            "MetricsReportingEnabled",
            "BlockThirdPartyCookies",
            "SafeBrowsingProtectionLevel",
        ] {
            assert!(
                Browser::Chrome.supports_policy(key),
                "{key} is a shared policy"
            );
        }
    }

    #[test]
    fn i18n_key_matches_fluent_message_keys() {
        assert_eq!(Browser::Brave.i18n_key(), "browser-brave");
        assert_eq!(Browser::Chrome.i18n_key(), "browser-chrome");
    }
}
