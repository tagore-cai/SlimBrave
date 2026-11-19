//! 用例：清洗从浏览器读回的原始载荷，仅保留受管键并校验类型。

use std::collections::BTreeMap;

use crate::domain::catalog::init as catalog;
use crate::domain::catalog::PolicyValue;
use crate::domain::{Browser, PlatformKind};

use super::{expected_value, feature_index, permission_index, Payload, RawValue, WriteValue};

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

pub fn sanitize_payload(
    raw: &BTreeMap<String, RawValue>,
    platform: PlatformKind,
    browser: Browser,
) -> (Payload, Vec<String>) {
    let managed = catalog().managed_keys();
    let mut cleaned = Payload::new();
    let mut warnings = Vec::new();

    for (key, value) in raw {
        if !managed.contains(key) || !browser.supports_policy(key) {
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
