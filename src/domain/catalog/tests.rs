use super::model::Catalog;
use super::schema::{JsonValue, RawPresets, UserFeature};
use super::store::BUILTIN_CATALOG;
use super::{PolicyValue, UserCatalog};
use crate::domain::Browser;

const REFERENCE: &[(&str, &str, bool, bool)] = &[
    (
        "MetricsReportingEnabled",
        "Disable Metrics Reporting",
        true,
        true,
    ),
    (
        "SafeBrowsingExtendedReportingEnabled",
        "Disable Safe Browsing Reporting",
        true,
        true,
    ),
    (
        "UrlKeyedAnonymizedDataCollectionEnabled",
        "Disable URL Data Collection",
        true,
        true,
    ),
    (
        "FeedbackSurveysEnabled",
        "Disable Feedback Surveys",
        true,
        true,
    ),
    ("BraveP3AEnabled", "Disable P3A Telemetry", true, true),
    (
        "BraveStatsPingEnabled",
        "Disable Daily Stats Ping",
        true,
        true,
    ),
    (
        "BraveWebDiscoveryEnabled",
        "Disable Web Discovery",
        true,
        true,
    ),
    (
        "AutofillAddressEnabled",
        "Disable Autofill (Addresses)",
        true,
        false,
    ),
    (
        "AutofillCreditCardEnabled",
        "Disable Autofill (Credit Cards)",
        true,
        false,
    ),
    (
        "PasswordManagerEnabled",
        "Disable Password Manager",
        true,
        false,
    ),
    ("BrowserSignin", "Disable Browser Sign-in", true, false),
    ("WebRtcIPHandling", "Disable WebRTC IP Leak", true, true),
    ("QuicAllowed", "Disable QUIC Protocol", false, true),
    (
        "BlockThirdPartyCookies",
        "Block Third Party Cookies",
        true,
        true,
    ),
    ("EnableDoNotTrack", "Enable Do Not Track", true, true),
    (
        "GlobalPrivacyControlEnabled",
        "Enable Global Privacy Control",
        true,
        true,
    ),
    ("BraveDeAMPEnabled", "Enable De-AMP", true, true),
    ("BraveDebouncingEnabled", "Enable Debouncing", true, true),
    (
        "BraveTrackersStrippingEnabled",
        "Strip URL Trackers",
        true,
        true,
    ),
    (
        "ReduceAcceptLanguage",
        "Reduce Language Fingerprinting",
        true,
        true,
    ),
    (
        "ForceGoogleSafeSearch",
        "Force Google SafeSearch",
        false,
        true,
    ),
    ("IPFSEnabled", "Disable IPFS", true, true),
    (
        "IncognitoModeAvailability",
        "Force Incognito Mode",
        false,
        false,
    ),
    (
        "PromptForDownloadLocation",
        "Force Download Prompts",
        true,
        true,
    ),
    (
        "ClearBrowsingDataOnExitList",
        "Clear Data on Exit",
        true,
        false,
    ),
    ("HttpsOnlyMode", "Force HTTPS-Only Mode", true, true),
    ("BraveRewardsDisabled", "Disable Brave Rewards", true, true),
    ("BraveWalletDisabled", "Disable Brave Wallet", true, true),
    ("BraveVPNDisabled", "Disable Brave VPN", true, true),
    ("BraveAIChatEnabled", "Disable Brave AI Chat", true, true),
    ("TorDisabled", "Disable Tor", true, true),
    ("SyncDisabled", "Disable Sync", false, true),
    ("BraveNewsDisabled", "Disable Brave News", true, true),
    ("BraveTalkDisabled", "Disable Brave Talk", true, true),
    ("BraveSpeedreaderEnabled", "Disable Speedreader", true, true),
    (
        "BraveWaybackMachineEnabled",
        "Disable Wayback Machine Prompts",
        true,
        true,
    ),
    (
        "BackgroundModeEnabled",
        "Disable Background Mode",
        true,
        true,
    ),
    (
        "MediaRecommendationsEnabled",
        "Disable Media Recommendations",
        true,
        false,
    ),
    ("ShoppingListEnabled", "Disable Shopping List", true, false),
    (
        "AlwaysOpenPdfExternally",
        "Always Open PDF Externally",
        true,
        true,
    ),
    ("TranslateEnabled", "Disable Translate", false, false),
    ("SpellcheckEnabled", "Disable Spellcheck", false, false),
    ("PromotionsEnabled", "Disable Promotions", true, false),
    (
        "SearchSuggestEnabled",
        "Disable Search Suggestions",
        true,
        false,
    ),
    ("PrintingEnabled", "Disable Printing", false, false),
    (
        "DefaultBrowserSettingEnabled",
        "Disable Default Browser Prompt",
        true,
        false,
    ),
    (
        "DeveloperToolsDisabled",
        "Disable Developer Tools",
        false,
        true,
    ),
    ("BravePlaylistEnabled", "Disable Brave Playlist", true, true),
];

#[test]
fn builtin_catalog_matches_reference() {
    let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid builtin catalog");
    assert_eq!(catalog.features.len(), REFERENCE.len());
    for (expected, actual) in REFERENCE.iter().zip(catalog.features.iter()) {
        assert_eq!(actual.key, expected.0, "key mismatch");
        assert_eq!(actual.name, expected.1, "name mismatch for {}", actual.key);
        assert_eq!(
            (actual.privacy, actual.security),
            (expected.2, expected.3),
            "suggestion flags mismatch for {}",
            actual.key
        );
    }
    assert_eq!(catalog.permissions.len(), 15);
    assert_eq!(catalog.privacy_keys.len(), 40);
    assert_eq!(catalog.security_keys.len(), 29);
}

#[test]
fn catalog_counts_match_reference() {
    let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid builtin catalog");
    let count = |section: &str| {
        catalog
            .features
            .iter()
            .filter(|f| f.section == section)
            .count()
    };
    assert_eq!(count("telemetry"), 7);
    assert_eq!(count("privacy"), 19);
    assert_eq!(count("brave"), 10);
    assert_eq!(count("perf"), 12);
}

#[test]
fn user_layer_overrides_adds_and_removes() {
    let mut catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid");
    let user = UserCatalog {
        features: vec![
            UserFeature {
                key: "MetricsReportingEnabled".to_owned(),
                name: Some("My Metric Toggle".to_owned()),
                tooltip: None,
                value: None,
                windows_value: None,
                privacy: Some(false),
                security: None,
                section: None,
            },
            UserFeature {
                key: "MyCustomPolicy".to_owned(),
                name: Some("Custom Policy".to_owned()),
                tooltip: Some("A user-defined policy".to_owned()),
                value: Some(JsonValue::Bool(true)),
                windows_value: None,
                privacy: Some(true),
                security: Some(true),
                section: Some("privacy".to_owned()),
            },
        ],
        permissions: vec![],
        remove: vec!["AutofillAddressEnabled".to_owned()],
        presets: RawPresets::default(),
    };
    catalog.merge_user(&user);

    let feature = catalog.feature_by_key("MetricsReportingEnabled").unwrap();
    assert_eq!(feature.name, "My Metric Toggle");
    assert_eq!(
        feature.tooltip,
        "Stops Brave from sending anonymous usage and crash reports."
    );
    assert!(!feature.privacy);

    assert!(catalog.feature_by_key("AutofillAddressEnabled").is_none());
    let custom = catalog.feature_by_key("MyCustomPolicy").unwrap();
    assert_eq!(custom.value, PolicyValue::Bool(true));
    assert_eq!(custom.section, "privacy");
    assert_eq!(catalog.features.len(), 48);
}

#[test]
fn managed_keys_covers_catalog_and_special_keys() {
    let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid");
    let keys = catalog.managed_keys();
    assert!(keys.contains("MetricsReportingEnabled"));
    assert!(keys.contains("DefaultGeolocationSetting"));
    assert!(keys.contains("SafeBrowsingProtectionLevel"));
    assert!(keys.contains("DnsOverHttpsTemplates"));
}

#[test]
fn features_for_filters_by_browser_and_keeps_original_indices() {
    let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid");

    let brave: Vec<_> = catalog.features_for(Browser::Brave).collect();
    let chrome: Vec<_> = catalog.features_for(Browser::Chrome).collect();

    assert_eq!(brave.len(), catalog.features.len());
    assert_eq!(chrome.len(), catalog.features.len() - 17);

    // Chrome 结果中不含 Brave 专属键
    assert!(chrome
        .iter()
        .all(|(_, f)| !crate::domain::BRAVE_ONLY_KEYS.contains(&f.key.as_str())));
    // 共享策略在两者中都在，且原始索引一致
    for key in ["MetricsReportingEnabled", "BlockThirdPartyCookies"] {
        let bi = brave
            .iter()
            .find(|(_, f)| f.key == key)
            .map(|(i, _)| *i)
            .unwrap();
        let ci = chrome
            .iter()
            .find(|(_, f)| f.key == key)
            .map(|(i, _)| *i)
            .unwrap();
        assert_eq!(bi, ci, "{key} index must stay stable across browsers");
    }
}
