//! 用例：把 [`UiState`](super::super::UiState) 编译成 [`ApplyPlan`](super::ApplyPlan)。

use crate::domain::catalog::init as catalog;
use crate::domain::state::{DNS_MODE_OPTIONS, NOT_SET, SAFE_BROWSING_OPTIONS};
use crate::domain::{Browser, PlatformKind};

use super::{expected_value, ApplyPlan, WriteValue};

pub fn build_apply_plan(
    state: &crate::domain::UiState,
    platform: PlatformKind,
    browser: Browser,
) -> ApplyPlan {
    let mut plan = ApplyPlan::default();

    for (index, feature) in catalog().features_for(browser) {
        if state.checked.get(index).copied().unwrap_or(false) {
            plan.write.push((
                feature.key.to_string(),
                expected_value(feature, platform).into(),
            ));
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
                plan.delete
                    .push("DefaultFileSystemWriteGuardSetting".to_string());
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

    match state
        .safe_browsing
        .map(|index| SAFE_BROWSING_OPTIONS[index])
    {
        Some("On") => plan.write.push((
            "SafeBrowsingProtectionLevel".to_string(),
            WriteValue::Int(1),
        )),
        Some("Off") => plan.write.push((
            "SafeBrowsingProtectionLevel".to_string(),
            WriteValue::Int(0),
        )),
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
