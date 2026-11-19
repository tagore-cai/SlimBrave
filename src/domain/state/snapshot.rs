//! 状态快照：与 Python 版兼容的 PascalCase JSON 导入导出格式。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{UiState, DNS_MODE_OPTIONS, SAFE_BROWSING_OPTIONS};
use crate::domain::catalog::init as catalog;
use crate::domain::Browser;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StateSnapshot {
    pub features: Vec<String>,
    pub permissions: BTreeMap<String, String>,
    pub safe_browsing: String,
    pub dns_mode: String,
    pub dns_template: String,
}

impl UiState {
    pub fn to_snapshot(&self) -> StateSnapshot {
        let features = catalog()
            .features
            .iter()
            .enumerate()
            .filter(|(index, _)| self.checked.get(*index).copied().unwrap_or(false))
            .map(|(_, feature)| feature.key.clone())
            .collect();

        let permissions = catalog()
            .permissions
            .iter()
            .enumerate()
            .filter(|(index, _)| self.permissions.get(*index).copied().unwrap_or(0) != 0)
            .filter_map(|(index, permission)| {
                let selection = self.permissions.get(index).copied().unwrap_or(0);
                let option = permission.options.get(selection)?.clone();
                Some((permission.key.clone(), option))
            })
            .collect();

        StateSnapshot {
            features,
            permissions,
            safe_browsing: self
                .safe_browsing
                .map(|index| SAFE_BROWSING_OPTIONS[index].to_string())
                .unwrap_or_default(),
            dns_mode: DNS_MODE_OPTIONS[self.dns_mode].to_string(),
            dns_template: self.dns_template.clone(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: &StateSnapshot, browser: Browser) {
        *self = UiState::default();

        for key in &snapshot.features {
            if !browser.supports_policy(key) {
                continue;
            }
            if let Some(feature) = catalog().feature_by_key(key) {
                if let Some(index) = catalog().features.iter().position(|f| f.key == feature.key) {
                    self.checked[index] = true;
                }
            }
        }

        for (key, selection) in &snapshot.permissions {
            if let Some(permission) = catalog().permission_by_key(key) {
                if let Some(index) = catalog()
                    .permissions
                    .iter()
                    .position(|p| p.key == permission.key)
                {
                    if let Some(option) = permission.options.iter().position(|o| o == selection) {
                        self.permissions[index] = option;
                    }
                }
            }
        }

        if !snapshot.safe_browsing.is_empty() {
            self.safe_browsing = SAFE_BROWSING_OPTIONS
                .iter()
                .position(|option| *option == snapshot.safe_browsing);
        }
        if !snapshot.dns_mode.is_empty() {
            self.dns_mode = DNS_MODE_OPTIONS
                .iter()
                .position(|option| *option == snapshot.dns_mode)
                .unwrap_or(0);
        }
        self.dns_template = snapshot.dns_template.clone();
    }
}
