use super::*;

#[test]
fn parse_yaml_scalars_and_blocks() {
    let yaml = r#"caption: Enable reporting of usage data
desc: |-
  When this policy is Enabled, data is reported.

  Second paragraph.
type: boolean
supported_on:
- chrome.win:50-
features:
  dynamic_refresh: true
"#;
    let parsed = super::yaml::parse_simple_yaml(yaml);
    assert_eq!(
        parsed.get("caption").map(String::as_str),
        Some("Enable reporting of usage data")
    );
    assert!(parsed.get("desc").unwrap().contains("Second paragraph"));
    assert_eq!(parsed.get("type").map(String::as_str), Some("boolean"));
    assert!(parsed
        .get("supported_on")
        .unwrap()
        .contains("chrome.win:50-"));
}

#[test]
fn clean_desc_strips_markup_and_renames_product() {
    let desc =
        "About <ph name=\"PRODUCT_NAME\">$1<ex>Google Chrome</ex></ph> data\nfor $2 reports.";
    let cleaned = super::yaml::clean_desc(desc);
    assert!(!cleaned.contains('<'));
    assert!(cleaned.contains("Brave"));
    assert!(!cleaned.contains("$"));
}

#[test]
fn clean_desc_handles_empty() {
    assert_eq!(super::yaml::clean_desc(""), "");
}

#[test]
#[ignore = "requires network"]
fn fetch_policies_network_end_to_end() {
    let brave = fetch_policies(Browser::Brave).expect("brave fetch should succeed");
    assert!(
        brave >= 43,
        "brave expected at least 43 matched policies, got {brave}"
    );
    let path = remote_path(Browser::Brave).unwrap();
    let json = std::fs::read_to_string(&path).expect("brave remote catalog written");
    let remote: crate::domain::catalog::RemoteCatalog =
        serde_json::from_str(&json).expect("remote catalog parses");
    assert!(
        remote.policies.contains_key("BraveRewardsDisabled"),
        "brave-specific policy missing"
    );
    assert!(remote.policies.contains_key("MetricsReportingEnabled"));

    let chrome = fetch_policies(Browser::Chrome).expect("chrome fetch should succeed");
    assert!(
        chrome >= 28,
        "chrome expected at least 28 matched policies, got {chrome}"
    );
    let path = remote_path(Browser::Chrome).unwrap();
    let json = std::fs::read_to_string(&path).expect("chrome remote catalog written");
    let remote: crate::domain::catalog::RemoteCatalog =
        serde_json::from_str(&json).expect("remote catalog parses");
    assert!(
        !remote.policies.contains_key("BraveRewardsDisabled"),
        "brave policy must not leak into chrome"
    );
}

#[test]
fn fetch_scope_only_targets_current_browsers_policies() {
    use crate::domain::{BRAVE_ONLY_KEYS, EDGE_ONLY_KEYS};

    let (brave_keys, brave_existing) = super::fetch_scope(Browser::Brave);
    let (chrome_keys, chrome_existing) = super::fetch_scope(Browser::Chrome);
    let (edge_keys, _) = super::fetch_scope(Browser::Edge);

    let total = BRAVE_ONLY_KEYS.len() + EDGE_ONLY_KEYS.len() + 31;
    assert_eq!(brave_keys.len(), total - EDGE_ONLY_KEYS.len());
    assert_eq!(chrome_keys.len(), 31);
    assert_eq!(edge_keys.len(), total - BRAVE_ONLY_KEYS.len());

    for key in BRAVE_ONLY_KEYS {
        assert!(
            brave_keys.iter().any(|k| k == key),
            "{key} fetched for Brave"
        );
        assert!(
            !chrome_keys.iter().any(|k| k == key),
            "{key} must not be fetched for Chrome"
        );
        assert!(
            !chrome_existing.contains_key(*key),
            "{key} must not be in Chrome fallback"
        );
    }
    // shared keys present in both scopes
    assert!(chrome_keys.iter().any(|k| k == "MetricsReportingEnabled"));
    assert!(brave_existing.contains_key("MetricsReportingEnabled"));
}
