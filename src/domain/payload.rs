use std::collections::BTreeMap;

use crate::domain::catalog::{init as catalog, PolicyValue};
use crate::domain::state::{DNS_MODE_OPTIONS, NOT_SET, SAFE_BROWSING_OPTIONS, UiState};
use crate::domain::PlatformKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawValue {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteValue {
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<String>),
}

pub type Payload = BTreeMap<String, WriteValue>;

impl From<PolicyValue> for WriteValue {
    fn from(value: PolicyValue) -> Self {
        match value {
            PolicyValue::Bool(value) => WriteValue::Bool(value),
            PolicyValue::Int(value) => WriteValue::Int(value),
            PolicyValue::Str(value) => WriteValue::Str(value.to_string()),
            PolicyValue::Array(items) => WriteValue::Array(
                items
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>(),
            ),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ApplyPlan {
    pub write: Vec<(String, WriteValue)>,
    pub delete: Vec<String>,
}

fn expected_value(feature: &crate::domain::catalog::Feature, platform: PlatformKind) -> PolicyValue {
    match platform {
        PlatformKind::Windows => feature
            .windows_value
            .as_ref()
            .unwrap_or(&feature.value)
            .clone(),
        PlatformKind::MacOs => feature.value.clone(),
    }
}

fn coerce_bool(value: &RawValue) -> Option<bool> {
    match value {
        RawValue::Bool(value) => Some(*value),
        RawValue::Int(value) => Some(*value != 0),
        RawValue::Str(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        },
        RawValue::Array(_) => None,
    }
}

fn raw_to_int(value: &RawValue) -> Option<i64> {
    match value {
        RawValue::Int(value) => Some(*value),
        RawValue::Bool(value) => Some(i64::from(*value)),
        RawValue::Str(value) => value.trim().parse::<i64>().ok(),
        RawValue::Array(_) => None,
    }
}

fn stringify(value: &RawValue) -> String {
    match value {
        RawValue::Str(value) => value.clone(),
        RawValue::Int(value) => value.to_string(),
        RawValue::Bool(value) => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        RawValue::Array(items) => items.join(","),
    }
}

fn feature_index(key: &str) -> Option<usize> {
    catalog().features.iter().position(|feature| feature.key == key)
}

fn permission_index(key: &str) -> Option<usize> {
    catalog()
        .permissions
        .iter()
        .position(|permission| permission.key == key)
}

pub fn build_apply_plan(state: &UiState, platform: PlatformKind) -> ApplyPlan {
    let mut plan = ApplyPlan::default();

    for (index, feature) in catalog().features.iter().enumerate() {
        if state.checked.get(index).copied().unwrap_or(false) {
            plan.write
                .push((feature.key.to_string(), expected_value(feature, platform).into()));
        } else {
            plan.delete.push(feature.key.to_string());
        }
    }

    for (index, permission) in catalog().permissions.iter().enumerate() {
        let Some(&selection_idx) = state.permissions.get(index) else {
            continue;
        };
        let Some(selection) = permission.options.get(selection_idx) else {
            continue;
        };
        let selection = selection.clone();
        let key = permission.key.clone();

        if selection == NOT_SET {
            plan.delete.push(key.clone());
            if key == "DefaultFileSystemReadGuardSetting" {
                plan.delete.push("DefaultFileSystemWriteGuardSetting".to_string());
            }
            continue;
        }

        if key == "PaymentMethodQueryEnabled" {
            plan.write
                .push((key.clone(), WriteValue::Bool(selection == "Allow")));
        } else {
            let value = match selection.as_str() {
                "Ask" => 3,
                "Block" => 2,
                _ => 1,
            };
            plan.write.push((key.clone(), WriteValue::Int(value)));
            if key == "DefaultFileSystemReadGuardSetting" {
                plan.write.push((
                    "DefaultFileSystemWriteGuardSetting".to_string(),
                    WriteValue::Int(value),
                ));
            }
        }
    }

    match state.safe_browsing.map(|index| SAFE_BROWSING_OPTIONS[index]) {
        Some("On") => plan
            .write
            .push(("SafeBrowsingProtectionLevel".to_string(), WriteValue::Int(1))),
        Some("Off") => plan
            .write
            .push(("SafeBrowsingProtectionLevel".to_string(), WriteValue::Int(0))),
        _ => plan.delete.push("SafeBrowsingProtectionLevel".to_string()),
    }

    match DNS_MODE_OPTIONS[state.dns_mode] {
        "Automatic" => {
            plan.write.push((
                "DnsOverHttpsMode".to_string(),
                WriteValue::Str("automatic".to_string()),
            ));
            plan.delete.push("DnsOverHttpsTemplates".to_string());
        }
        "Off" => {
            plan.write.push((
                "DnsOverHttpsMode".to_string(),
                WriteValue::Str("off".to_string()),
            ));
            plan.delete.push("DnsOverHttpsTemplates".to_string());
        }
        "Secure" | "Custom" => {
            plan.write.push((
                "DnsOverHttpsMode".to_string(),
                WriteValue::Str("secure".to_string()),
            ));
            let template = state.dns_template.trim();
            if template.is_empty() {
                plan.delete.push("DnsOverHttpsTemplates".to_string());
            } else {
                plan.write.push((
                    "DnsOverHttpsTemplates".to_string(),
                    WriteValue::Str(template.to_string()),
                ));
            }
        }
        _ => {
            plan.delete.push("DnsOverHttpsMode".to_string());
            plan.delete.push("DnsOverHttpsTemplates".to_string());
        }
    }

    plan.delete.sort();
    plan.delete.dedup();
    plan
}

pub fn sanitize_payload(
    raw: &BTreeMap<String, RawValue>,
    platform: PlatformKind,
) -> (Payload, Vec<String>) {
    let managed = catalog().managed_keys();
    let mut cleaned = Payload::new();
    let mut warnings = Vec::new();

    for (key, value) in raw {
        if !managed.contains(key) {
            continue;
        }

        if let Some(feature) = feature_index(key) {
            let feature = &catalog().features[feature];
            match expected_value(feature, platform) {
                PolicyValue::Bool(_) => match coerce_bool(value) {
                    Some(value) => {
                        cleaned.insert(key.clone(), WriteValue::Bool(value));
                    }
                    None => warnings.push(format!("Skipped {key}: expected bool")),
                },
                PolicyValue::Int(_) => match raw_to_int(value) {
                    Some(value) => {
                        cleaned.insert(key.clone(), WriteValue::Int(value));
                    }
                    None => warnings.push(format!("Skipped {key}: expected int")),
                },
                PolicyValue::Str(_) => {
                    cleaned.insert(key.clone(), WriteValue::Str(stringify(value)));
                }
                PolicyValue::Array(_) => match value {
                    RawValue::Array(items) => {
                        cleaned.insert(
                            key.clone(),
                            WriteValue::Array(items.iter().map(|item| item.to_string()).collect()),
                        );
                    }
                    _ => warnings.push(format!("Skipped {key}: expected array")),
                },
            }
            continue;
        }

        if let Some(index) = permission_index(key) {
            let permission = &catalog().permissions[index];
            if permission.key == "PaymentMethodQueryEnabled" {
                match coerce_bool(value) {
                    Some(value) => {
                        cleaned.insert(key.clone(), WriteValue::Bool(value));
                    }
                    None => warnings.push(format!("Skipped {key}: invalid value")),
                }
            } else if let Some(value) = raw_to_int(value) {
                if matches!(value, 1..=3) {
                    cleaned.insert(key.clone(), WriteValue::Int(value));
                } else {
                    warnings.push(format!("Skipped {key}: invalid value {value}"));
                }
            } else {
                warnings.push(format!("Skipped {key}: invalid value"));
            }
            continue;
        }

        match key.as_str() {
            "SafeBrowsingProtectionLevel" => match raw_to_int(value) {
                Some(value @ (0 | 1)) => {
                    cleaned.insert(key.clone(), WriteValue::Int(value));
                }
                Some(value) => warnings.push(format!("Skipped {key}: invalid value {value}")),
                None => warnings.push(format!("Skipped {key}: invalid value")),
            },
            "DnsOverHttpsMode" => {
                if let RawValue::Str(value) = value {
                    let normalized = value.trim().to_ascii_lowercase();
                    if matches!(normalized.as_str(), "automatic" | "off" | "secure") {
                        cleaned.insert(key.clone(), WriteValue::Str(normalized));
                    } else {
                        warnings.push(format!("Skipped {key}: invalid value"));
                    }
                } else {
                    warnings.push(format!("Skipped {key}: invalid value"));
                }
            }
            "DnsOverHttpsTemplates" => {
                cleaned.insert(key.clone(), WriteValue::Str(stringify(value)));
            }
            "DefaultFileSystemWriteGuardSetting" => match raw_to_int(value) {
                Some(value @ (1..=3)) => {
                    cleaned.insert(key.clone(), WriteValue::Int(value));
                }
                Some(value) => warnings.push(format!("Skipped {key}: invalid value {value}")),
                None => warnings.push(format!("Skipped {key}: invalid value")),
            },
            _ => {}
        }
    }

    if cleaned.contains_key("DefaultFileSystemReadGuardSetting")
        && !cleaned.contains_key("DefaultFileSystemWriteGuardSetting")
    {
        if let Some(WriteValue::Int(value)) = cleaned.get("DefaultFileSystemReadGuardSetting") {
            cleaned.insert(
                "DefaultFileSystemWriteGuardSetting".to_string(),
                WriteValue::Int(*value),
            );
        }
    }

    (cleaned, warnings)
}

pub fn apply_payload_to_ui(state: &mut UiState, payload: &Payload, platform: PlatformKind) {
    *state = UiState::default();

    for feature in catalog().features.iter() {
        let Some(value) = payload.get(&feature.key) else {
            continue;
        };
        let expected = expected_value(feature, platform);
        let matches = match (expected, value) {
            (PolicyValue::Bool(expected), WriteValue::Bool(actual)) => expected == *actual,
            (PolicyValue::Int(expected), WriteValue::Int(actual)) => expected == *actual,
            (PolicyValue::Str(expected), WriteValue::Str(actual)) => expected == *actual,
            (PolicyValue::Array(_), WriteValue::Array(actual)) => !actual.is_empty(),
            _ => false,
        };
        if matches {
            if let Some(index) = feature_index(&feature.key) {
                state.checked[index] = true;
            }
        }
    }

    for (index, permission) in catalog().permissions.iter().enumerate() {
        let Some(value) = payload.get(&permission.key) else {
            continue;
        };
        let selection = if permission.key == "PaymentMethodQueryEnabled" {
            match value {
                WriteValue::Bool(true) => "Allow",
                WriteValue::Bool(false) => "Block",
                _ => continue,
            }
        } else {
            match value {
                WriteValue::Int(3) => "Ask",
                WriteValue::Int(2) => "Block",
                WriteValue::Int(1) => "Allow",
                _ => continue,
            }
        };
        if let Some(option) = permission.options.iter().position(|o| *o == selection) {
            state.permissions[index] = option;
        }
    }

    match payload.get("SafeBrowsingProtectionLevel") {
        Some(WriteValue::Int(1)) => state.safe_browsing = Some(0),
        Some(WriteValue::Int(0)) => state.safe_browsing = Some(1),
        _ => {}
    }

    if let Some(WriteValue::Str(mode)) = payload.get("DnsOverHttpsMode") {
        let has_template = payload
            .get("DnsOverHttpsTemplates")
            .map(|value| !matches!(value, WriteValue::Str(value) if value.trim().is_empty()))
            .unwrap_or(false);
        state.dns_mode = match (mode.as_str(), has_template) {
            ("automatic", _) => 0,
            ("off", _) => 1,
            ("secure", true) => 3,
            ("secure", false) => 2,
            _ => 0,
        };
    }

    if let Some(WriteValue::Str(template)) = payload.get("DnsOverHttpsTemplates") {
        state.dns_template = template.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PlatformKind, Preset};

    fn mac() -> PlatformKind {
        PlatformKind::MacOs
    }

    #[test]
    fn plan_writes_checked_and_deletes_unchecked() {
        let mut state = UiState::default();
        state.checked[0] = true;
        state.checked[4] = true;
        state.permissions[0] = 2;
        state.safe_browsing = Some(0);
        state.dns_mode = 1;

        let plan = build_apply_plan(&state, mac());

        assert!(plan
            .write
            .contains(&("MetricsReportingEnabled".to_string(), WriteValue::Bool(false))));
        assert!(plan
            .write
            .contains(&("DefaultGeolocationSetting".to_string(), WriteValue::Int(2))));
        assert!(plan
            .write
            .contains(&("SafeBrowsingProtectionLevel".to_string(), WriteValue::Int(1))));
        assert!(plan
            .write
            .contains(&("DnsOverHttpsMode".to_string(), WriteValue::Str("off".to_string()))));
        assert!(plan.delete.contains(&"AutofillAddressEnabled".to_string()));
        assert!(plan.delete.contains(&"DnsOverHttpsTemplates".to_string()));
    }

    #[test]
    fn plan_uses_windows_value_on_windows() {
        let mut state = UiState::default();
        let index = feature_index("BraveP3AEnabled").unwrap();
        state.checked[index] = true;

        let mac_plan = build_apply_plan(&state, PlatformKind::MacOs);
        assert!(mac_plan
            .write
            .contains(&("BraveP3AEnabled".to_string(), WriteValue::Bool(false))));

        let win_plan = build_apply_plan(&state, PlatformKind::Windows);
        assert!(win_plan
            .write
            .contains(&("BraveP3AEnabled".to_string(), WriteValue::Str("Disabled".to_string()))));
    }

    #[test]
    fn sanitize_accepts_known_keys_and_reports_warnings() {
        let mut raw = BTreeMap::new();
        raw.insert("MetricsReportingEnabled".to_string(), RawValue::Bool(false));
        raw.insert("BrowserSignin".to_string(), RawValue::Int(0));
        raw.insert("NotARealKey".to_string(), RawValue::Int(1));

        let (cleaned, warnings) = sanitize_payload(&raw, mac());
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

        let (cleaned, warnings) = sanitize_payload(&raw, mac());
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
        raw.insert("BrowserSignin".to_string(), RawValue::Array(vec!["1".to_string()]));
        raw.insert("DnsOverHttpsMode".to_string(), RawValue::Str("bogus".to_string()));

        let (cleaned, warnings) = sanitize_payload(&raw, mac());
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

        let (cleaned, _) = sanitize_payload(&raw, mac());
        assert_eq!(
            cleaned.get("DefaultFileSystemWriteGuardSetting"),
            Some(&WriteValue::Int(2))
        );
    }

    #[test]
    fn payload_round_trips_through_ui() {
        let mut state = UiState::default();
        state.checked[1] = true;
        state.permissions[3] = 2;
        state.safe_browsing = Some(1);
        state.dns_mode = 3;
        state.dns_template = "https://dns.example/dns-query".to_string();

        let plan = build_apply_plan(&state, mac());
        let payload: Payload = plan.write.into_iter().collect();

        let mut restored = UiState::default();
        apply_payload_to_ui(&mut restored, &payload, mac());

        assert_eq!(restored.checked, state.checked);
        assert_eq!(restored.permissions, state.permissions);
        assert_eq!(restored.safe_browsing, state.safe_browsing);
        assert_eq!(restored.dns_mode, state.dns_mode);
        assert_eq!(restored.dns_template, state.dns_template);
    }

    #[test]
    fn privacy_preset_matches_expected_state() {
        let mut state = UiState::default();
        state.apply_preset(Preset::Privacy);

        assert!(state.checked[feature_index("MetricsReportingEnabled").unwrap()]);
        assert!(state.checked[feature_index("BlockThirdPartyCookies").unwrap()]);
        assert!(!state.checked[feature_index("QuicAllowed").unwrap()]);
        assert_eq!(state.safe_browsing, Some(1));
        assert_eq!(state.dns_mode, 1);
        assert_eq!(state.permissions[permission_index("DefaultGeolocationSetting").unwrap()], 2);
        assert_eq!(state.permissions[permission_index("DefaultJavaScriptSetting").unwrap()], 1);
    }
}
