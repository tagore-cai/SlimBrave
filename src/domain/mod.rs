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
    Edge,
}

/// 仅 Brave 支持的策略键。
///
/// 依据：`Brave*` 前缀为 Brave 自有策略；`IPFSEnabled`（Brave 内置 IPFS）与
/// `TorDisabled`（Brave 内置 Tor 窗口）是 Brave 特有功能，均不在 Chromium
/// 的 `components/policy/resources/templates/policy_definitions` 标准定义中。
/// Chrome 不应显示或写入这些键。
/// 仅 Microsoft Edge 支持的策略键。
///
/// 依据：Microsoft Edge 官方策略文档中 `Edge*` 前缀策略。Edge 专属项不写入
/// Brave / Chrome；其策略路径全渠道共用，与 Brave/Chrome 的每渠道独立不同。
pub const EDGE_ONLY_KEYS: &[&str] = &[
    "EdgeCopilotEnabled",
    "EdgeWorkspacesEnabled",
    "EdgeBlockSignInEnabled",
    "EdgeDefaultProfileEnabled",
    "EdgeAllowedAccountOnly",
    "EdgeManagementEnabled",
    "EdgeDisableDialProtocolForCastDiscovery",
    "EdgeEntraCopilotPageContext",
    "EdgeOpenExternalLinksWithAppSpecifiedProfile",
    "EdgeManagementExtensionsFeedbackEnabled",
    "EdgeManagementPolicyOverridesPlatformPolicy",
    "EdgeManagementUserPolicyOverridesCloudMachinePolicy",
];

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
            Browser::Edge => "catalog.remote-edge.json",
        }
    }

    /// Fluent 消息键，用于把浏览器名插入动态文案。
    pub fn i18n_key(self) -> &'static str {
        match self {
            Browser::Chrome => "browser-chrome",
            Browser::Brave => "browser-brave",
            Browser::Edge => "browser-edge",
        }
    }

    /// 该策略键是否适用于本浏览器。
    /// Brave / Chrome 共享 Chromium 通用策略；Edge 有自己的专属集。
    /// 三者的专属键互不包含：Chrome 排除 Brave 与 Edge 专属，Brave 排除 Edge 专属，Edge 排除 Brave 专属。
    pub fn supports_policy(self, key: &str) -> bool {
        match self {
            Browser::Brave => !EDGE_ONLY_KEYS.contains(&key),
            Browser::Chrome => !BRAVE_ONLY_KEYS.contains(&key) && !EDGE_ONLY_KEYS.contains(&key),
            Browser::Edge => !BRAVE_ONLY_KEYS.contains(&key),
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
    fn edge_rejects_other_browsers_exclusive_policies() {
        for key in BRAVE_ONLY_KEYS {
            assert!(
                !Browser::Edge.supports_policy(key),
                "{key} must be hidden in Edge"
            );
        }
        for key in EDGE_ONLY_KEYS {
            assert!(
                !Browser::Brave.supports_policy(key),
                "{key} must be hidden in Brave"
            );
            assert!(
                !Browser::Chrome.supports_policy(key),
                "{key} must be hidden in Chrome"
            );
            assert!(
                Browser::Edge.supports_policy(key),
                "{key} must be shown in Edge"
            );
        }
    }

    #[test]
    fn i18n_key_matches_fluent_message_keys() {
        assert_eq!(Browser::Brave.i18n_key(), "browser-brave");
        assert_eq!(Browser::Chrome.i18n_key(), "browser-chrome");
        assert_eq!(Browser::Edge.i18n_key(), "browser-edge");
    }
}
