use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::domain::catalog::{init as catalog};

pub const NOT_SET: &str = "Not Set";
pub const SAFE_BROWSING_OPTIONS: [&str; 2] = ["On", "Off"];
pub const DNS_MODE_OPTIONS: [&str; 4] = ["Automatic", "Off", "Secure", "Custom"];

#[derive(Debug, Clone)]
pub struct UiState {
    pub checked: Vec<bool>,
    pub permissions: Vec<usize>,
    pub safe_browsing: Option<usize>,
    pub dns_mode: usize,
    pub dns_template: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            checked: vec![false; catalog().features.len()],
            permissions: vec![0; catalog().permissions.len()],
            safe_browsing: None,
            dns_mode: 0,
            dns_template: String::new(),
        }
    }
}

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
            .filter(|(index, _)| {
                self.permissions.get(*index).copied().unwrap_or(0) != 0
            })
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

    pub fn apply_snapshot(&mut self, snapshot: &StateSnapshot) {
        *self = UiState::default();

        for key in &snapshot.features {
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
                    if let Some(option) = permission
                        .options
                        .iter()
                        .position(|o| o == selection)
                    {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Privacy,
    Security,
}





impl UiState {
    pub fn apply_preset(&mut self, preset: Preset) {
        self.checked.fill(false);

        for (index, permission) in catalog().permissions.iter().enumerate() {
            let selection = match permission.name.as_str() {
                "JavaScript" => "Allow",
                "Camera" | "Microphone" => "Ask",
                "Images" => NOT_SET,
                _ => {
                    if permission.options.iter().any(|o| o == "Block") {
                        "Block"
                    } else {
                        NOT_SET
                    }
                }
            };
            self.permissions[index] = permission
                .options
                .iter()
                .position(|option| option == selection)
                .unwrap_or(0);
        }

        let keys = match preset {
            Preset::Privacy => &catalog().privacy_keys,
            Preset::Security => &catalog().security_keys,
        };
        for key in keys {
            if let Some(feature) = catalog().feature_by_key(key) {
                if let Some(index) = catalog().features.iter().position(|f| f.key == feature.key) {
                    if let Some(checked) = self.checked.get_mut(index) {
                        *checked = true;
                    }
                }
            }
        }

        match preset {
            Preset::Privacy => {
                self.safe_browsing = Some(1);
                self.dns_mode = 1;
                self.dns_template.clear();
            }
            Preset::Security => {
                self.safe_browsing = Some(0);
                self.dns_mode = 0;
                self.dns_template.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(keys, vec!["DnsMode", "DnsTemplate", "Features", "Permissions", "SafeBrowsing"]);
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
        restored.apply_snapshot(&parsed);

        assert_eq!(restored.checked, state.checked);
        assert_eq!(restored.permissions, state.permissions);
        assert_eq!(restored.safe_browsing, state.safe_browsing);
        assert_eq!(restored.dns_mode, state.dns_mode);
        assert_eq!(restored.dns_template, state.dns_template);
    }
}

#[cfg(test)]
mod mismatch_tests {
    use super::*;

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
        state.apply_preset(Preset::Privacy);
        assert!(state.checked.len() >= 1);
    }
}
