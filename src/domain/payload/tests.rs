use super::*;
use crate::domain::{Browser, Preset};

fn mac() -> PlatformKind {
    PlatformKind::MacOs
}

fn brave() -> Browser {
    Browser::Brave
}

#[test]
fn plan_writes_checked_and_deletes_unchecked() {
    let mut state = crate::domain::UiState::default();
    state.checked[0] = true;
    state.checked[4] = true;
    state.permissions[0] = 2;
    state.safe_browsing = Some(0);
    state.dns_mode = 1;

    let plan = build_apply_plan(&state, mac(), brave());

    assert!(plan.write.contains(&(
        "MetricsReportingEnabled".to_string(),
        WriteValue::Bool(false)
    )));
    assert!(plan
        .write
        .contains(&("DefaultGeolocationSetting".to_string(), WriteValue::Int(2))));
    assert!(plan.write.contains(&(
        "SafeBrowsingProtectionLevel".to_string(),
        WriteValue::Int(1)
    )));
    assert!(plan.write.contains(&(
        "DnsOverHttpsMode".to_string(),
        WriteValue::Str("off".to_string())
    )));
    assert!(plan.delete.contains(&"AutofillAddressEnabled".to_string()));
    assert!(plan.delete.contains(&"DnsOverHttpsTemplates".to_string()));
}

#[test]
fn plan_uses_windows_value_on_windows() {
    let mut state = crate::domain::UiState::default();
    let index = feature_index("BraveP3AEnabled").unwrap();
    state.checked[index] = true;

    let mac_plan = build_apply_plan(&state, PlatformKind::MacOs, brave());
    assert!(mac_plan
        .write
        .contains(&("BraveP3AEnabled".to_string(), WriteValue::Bool(false))));

    let win_plan = build_apply_plan(&state, PlatformKind::Windows, Browser::Brave);
    assert!(win_plan.write.contains(&(
        "BraveP3AEnabled".to_string(),
        WriteValue::Str("Disabled".to_string())
    )));
}

#[test]
fn sanitize_accepts_known_keys_and_reports_warnings() {
    let mut raw = BTreeMap::new();
    raw.insert("MetricsReportingEnabled".to_string(), RawValue::Bool(false));
    raw.insert("BrowserSignin".to_string(), RawValue::Int(0));
    raw.insert("NotARealKey".to_string(), RawValue::Int(1));

    let (cleaned, warnings) = sanitize_payload(&raw, mac(), brave());
    assert!(cleaned.contains_key("MetricsReportingEnabled"));
    assert!(cleaned.contains_key("BrowserSignin"));
    assert!(!cleaned.contains_key("NotARealKey"));
    assert!(warnings.is_empty());
}

#[test]
fn sanitize_coerces_like_python() {
    let mut raw = BTreeMap::new();
    raw.insert("BrowserSignin".to_string(), RawValue::Str("0".to_string()));
    raw.insert("BrowserSignin2".to_string(), RawValue::Bool(true));
    raw.insert("HttpsOnlyMode".to_string(), RawValue::Bool(true));
    raw.insert("DnsOverHttpsTemplates".to_string(), RawValue::Int(42));

    let (cleaned, warnings) = sanitize_payload(&raw, mac(), brave());
    assert_eq!(
        cleaned.get("BrowserSignin"),
        Some(&WriteValue::Int(0)),
        "numeric string should coerce to int"
    );
    assert_eq!(
        cleaned.get("HttpsOnlyMode"),
        Some(&WriteValue::Str("true".to_string())),
        "any value should coerce to string"
    );
    assert_eq!(
        cleaned.get("DnsOverHttpsTemplates"),
        Some(&WriteValue::Str("42".to_string()))
    );
    assert!(warnings.is_empty());
}

#[test]
fn sanitize_rejects_wrong_types() {
    let mut raw = BTreeMap::new();
    raw.insert(
        "MetricsReportingEnabled".to_string(),
        RawValue::Str("nope".to_string()),
    );
    raw.insert(
        "BrowserSignin".to_string(),
        RawValue::Array(vec!["1".to_string()]),
    );
    raw.insert(
        "DnsOverHttpsMode".to_string(),
        RawValue::Str("bogus".to_string()),
    );

    let (cleaned, warnings) = sanitize_payload(&raw, mac(), brave());
    assert!(cleaned.is_empty());
    assert_eq!(warnings.len(), 3);
}

#[test]
fn sanitize_mirrors_file_system_write_guard() {
    let mut raw = BTreeMap::new();
    raw.insert(
        "DefaultFileSystemReadGuardSetting".to_string(),
        RawValue::Int(2),
    );

    let (cleaned, _) = sanitize_payload(&raw, mac(), brave());
    assert_eq!(
        cleaned.get("DefaultFileSystemWriteGuardSetting"),
        Some(&WriteValue::Int(2))
    );
}

#[test]
fn payload_round_trips_through_ui() {
    let mut state = crate::domain::UiState::default();
    state.checked[1] = true;
    state.permissions[3] = 2;
    state.safe_browsing = Some(1);
    state.dns_mode = 3;
    state.dns_template = "https://dns.example/dns-query".to_string();

    let plan = build_apply_plan(&state, mac(), brave());
    let payload: Payload = plan.write.into_iter().collect();

    let mut restored = crate::domain::UiState::default();
    apply_payload_to_ui(&mut restored, &payload, mac(), brave());

    assert_eq!(restored.checked, state.checked);
    assert_eq!(restored.permissions, state.permissions);
    assert_eq!(restored.safe_browsing, state.safe_browsing);
    assert_eq!(restored.dns_mode, state.dns_mode);
    assert_eq!(restored.dns_template, state.dns_template);
}

#[test]
fn privacy_preset_matches_expected_state() {
    let mut state = crate::domain::UiState::default();
    state.apply_preset(Preset::Privacy, brave());

    assert!(state.checked[feature_index("MetricsReportingEnabled").unwrap()]);
    assert!(state.checked[feature_index("BlockThirdPartyCookies").unwrap()]);
    assert!(!state.checked[feature_index("QuicAllowed").unwrap()]);
    assert_eq!(state.safe_browsing, Some(1));
    assert_eq!(state.dns_mode, 1);
    assert_eq!(
        state.permissions[permission_index("DefaultGeolocationSetting").unwrap()],
        2
    );
    assert_eq!(
        state.permissions[permission_index("DefaultJavaScriptSetting").unwrap()],
        1
    );
}

#[test]
fn chrome_plan_never_writes_brave_only_policies() {
    let mut state = crate::domain::UiState::default();
    let brave_key_index = feature_index("BraveRewardsDisabled").unwrap();
    let shared_index = feature_index("MetricsReportingEnabled").unwrap();
    state.checked[brave_key_index] = true;
    state.checked[shared_index] = true;

    let chrome_plan = build_apply_plan(&state, mac(), Browser::Chrome);
    assert!(
        !chrome_plan
            .write
            .iter()
            .any(|(key, _)| key == "BraveRewardsDisabled"),
        "Chrome plan must not write Brave-only policies"
    );
    assert!(chrome_plan
        .write
        .iter()
        .any(|(key, _)| key == "MetricsReportingEnabled"));

    let brave_plan = build_apply_plan(&state, mac(), Browser::Brave);
    assert!(brave_plan
        .write
        .iter()
        .any(|(key, _)| key == "BraveRewardsDisabled"));
}

#[test]
fn chrome_preset_skips_brave_only_policies() {
    let mut state = crate::domain::UiState::default();
    state.apply_preset(Preset::Privacy, Browser::Chrome);
    let brave_key_index = feature_index("BraveP3AEnabled").unwrap();
    assert!(
        !state.checked[brave_key_index],
        "Chrome preset must not tick Brave-only policies"
    );
}
