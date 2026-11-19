use super::{Preset, StateSnapshot, UiState};
use crate::domain::catalog::init as catalog;
use crate::domain::Browser;

#[test]
fn snapshot_json_matches_python_export_format() {
    let mut state = UiState::default();
    state.checked[0] = true;
    state.permissions[0] = 2;
    state.safe_browsing = Some(0);
    state.dns_mode = 3;
    state.dns_template = "https://dns.example/dns-query".to_string();

    let json = serde_json::to_string_pretty(&state.to_snapshot()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let obj = parsed.as_object().unwrap();
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "DnsMode",
            "DnsTemplate",
            "Features",
            "Permissions",
            "SafeBrowsing"
        ]
    );
    assert_eq!(
        obj["Features"],
        serde_json::json!(["MetricsReportingEnabled"])
    );
    assert_eq!(
        obj["Permissions"],
        serde_json::json!({"DefaultGeolocationSetting": "Block"})
    );
    assert_eq!(obj["SafeBrowsing"], "On");
    assert_eq!(obj["DnsMode"], "Custom");
    assert_eq!(obj["DnsTemplate"], "https://dns.example/dns-query");
}

#[test]
fn snapshot_round_trips() {
    let mut state = UiState::default();
    state.checked[1] = true;
    state.permissions[3] = 2;
    state.safe_browsing = Some(1);
    state.dns_mode = 3;
    state.dns_template = "https://dns.example/dns-query".to_string();

    let snapshot = state.to_snapshot();
    let json = serde_json::to_string(&snapshot).unwrap();
    let parsed: StateSnapshot = serde_json::from_str(&json).unwrap();
    let mut restored = UiState::default();
    restored.apply_snapshot(&parsed, Browser::Brave);

    assert_eq!(restored.checked, state.checked);
    assert_eq!(restored.permissions, state.permissions);
    assert_eq!(restored.safe_browsing, state.safe_browsing);
    assert_eq!(restored.dns_mode, state.dns_mode);
    assert_eq!(restored.dns_template, state.dns_template);
}

#[test]
fn snapshot_survives_state_catalog_mismatch() {
    let mut state = UiState::default();
    state.checked = vec![true; 1000];
    state.permissions = vec![99; 500];
    let snapshot = state.to_snapshot();
    assert!(snapshot.features.len() <= catalog().features.len());
    assert!(snapshot.permissions.len() <= catalog().permissions.len());
}

#[test]
fn preset_survives_short_checked_vector() {
    let mut state = UiState::default();
    state.checked = vec![false; 1];
    state.apply_preset(Preset::Privacy, Browser::Brave);
    assert!(state.checked.len() >= 1);
}
