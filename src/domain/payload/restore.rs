//! 用例：把外部载荷回灌到 [`UiState`](super::super::UiState)。

use crate::domain::catalog::init as catalog;
use crate::domain::catalog::PolicyValue;
use crate::domain::state::UiState;
use crate::domain::{Browser, PlatformKind};

use super::{expected_value, Payload, WriteValue};

pub fn apply_payload_to_ui(
    state: &mut UiState,
    payload: &Payload,
    platform: PlatformKind,
    browser: Browser,
) {
    *state = UiState::default();

    for (index, feature) in catalog().features_for(browser) {
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
            state.checked[index] = true;
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
