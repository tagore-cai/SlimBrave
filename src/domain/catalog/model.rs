use std::collections::BTreeSet;

use crate::domain::Browser;

use super::schema::{RawCatalog, UserCatalog};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyValue {
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<String>),
}

#[allow(dead_code)]
impl PolicyValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            PolicyValue::Str(s) => Some(s.clone()),
            PolicyValue::Bool(b) => Some(b.to_string()),
            PolicyValue::Int(i) => Some(i.to_string()),
            PolicyValue::Array(a) => Some(a.join(",")),
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            PolicyValue::Int(i) => Some(*i),
            PolicyValue::Bool(b) => Some(i64::from(*b)),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PolicyValue::Bool(b) => Some(*b),
            PolicyValue::Int(i) => Some(*i != 0),
            PolicyValue::Str(s) => match s.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            },
            PolicyValue::Array(_) => None,
        }
    }

    pub fn as_array(&self) -> Option<Vec<String>> {
        match self {
            PolicyValue::Array(a) => Some(a.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub key: String,
    pub name: String,
    pub tooltip: String,
    pub value: PolicyValue,
    pub windows_value: Option<PolicyValue>,
    pub privacy: bool,
    pub security: bool,
    pub section: String,
}

#[derive(Clone, Debug)]
pub struct Permission {
    pub key: String,
    pub name: String,
    pub options: Vec<String>,
    pub tooltip: String,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub features: Vec<Feature>,
    pub permissions: Vec<Permission>,
    pub privacy_keys: Vec<String>,
    pub security_keys: Vec<String>,
}

impl Catalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let raw: RawCatalog = serde_json::from_str(json).map_err(|err| err.to_string())?;
        let mut features = Vec::with_capacity(raw.features.len());
        for feature in raw.features {
            features.push(Feature {
                key: feature.key,
                name: feature.name,
                tooltip: feature.tooltip,
                value: feature
                    .value
                    .map(PolicyValue::from)
                    .unwrap_or(PolicyValue::Bool(false)),
                windows_value: feature.windows_value.map(PolicyValue::from),
                privacy: feature.privacy,
                security: feature.security,
                section: if feature.section.is_empty() {
                    "custom".to_owned()
                } else {
                    feature.section
                },
            });
        }
        let permissions = raw
            .permissions
            .into_iter()
            .map(|p| Permission {
                key: p.key,
                name: p.name,
                options: p.options,
                tooltip: p.tooltip,
            })
            .collect();
        Ok(Self {
            features,
            permissions,
            privacy_keys: raw.presets.privacy,
            security_keys: raw.presets.security,
        })
    }

    /// 按浏览器过滤策略，返回 `(原始索引, 策略)`。
    ///
    /// 保留原始索引是为了让调用方能直接索引 `UiState.checked` / `permissions`
    /// （这些向量按完整目录长度分配，不随浏览器变化）。
    pub fn features_for(&self, browser: Browser) -> impl Iterator<Item = (usize, &Feature)> {
        self.features
            .iter()
            .enumerate()
            .filter(move |(_, feature)| browser.supports_policy(&feature.key))
    }

    pub fn feature_by_key(&self, key: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.key == key)
    }

    pub fn permission_by_key(&self, key: &str) -> Option<&Permission> {
        self.permissions.iter().find(|p| p.key == key)
    }

    pub fn managed_keys(&self) -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        for feature in &self.features {
            keys.insert(feature.key.clone());
        }
        for permission in &self.permissions {
            keys.insert(permission.key.clone());
        }
        keys.insert("DefaultFileSystemWriteGuardSetting".to_owned());
        keys.insert("SafeBrowsingProtectionLevel".to_owned());
        keys.insert("DnsOverHttpsMode".to_owned());
        keys.insert("DnsOverHttpsTemplates".to_owned());
        keys
    }

    pub fn merge_user(&mut self, user: &UserCatalog) {
        self.features.retain(|f| !user.remove.contains(&f.key));
        self.permissions.retain(|p| !user.remove.contains(&p.key));

        for raw in &user.features {
            if let Some(feature) = self.features.iter_mut().find(|f| f.key == raw.key) {
                if let Some(name) = &raw.name {
                    feature.name = name.clone();
                }
                if let Some(tooltip) = &raw.tooltip {
                    feature.tooltip = tooltip.clone();
                }
                if let Some(value) = &raw.value {
                    feature.value = value.clone().into();
                }
                if let Some(windows_value) = &raw.windows_value {
                    feature.windows_value = Some(windows_value.clone().into());
                }
                if let Some(privacy) = raw.privacy {
                    feature.privacy = privacy;
                }
                if let Some(security) = raw.security {
                    feature.security = security;
                }
                if let Some(section) = &raw.section {
                    feature.section = section.clone();
                }
            } else {
                self.features.push(Feature {
                    key: raw.key.clone(),
                    name: raw.name.clone().unwrap_or_else(|| raw.key.clone()),
                    tooltip: raw.tooltip.clone().unwrap_or_default(),
                    value: raw
                        .value
                        .clone()
                        .map(PolicyValue::from)
                        .unwrap_or(PolicyValue::Bool(false)),
                    windows_value: raw.windows_value.clone().map(PolicyValue::from),
                    privacy: raw.privacy.unwrap_or(false),
                    security: raw.security.unwrap_or(false),
                    section: raw.section.clone().unwrap_or_else(|| "custom".to_owned()),
                });
            }
        }

        for raw in &user.permissions {
            if let Some(permission) = self.permissions.iter_mut().find(|p| p.key == raw.key) {
                if let Some(name) = &raw.name {
                    permission.name = name.clone();
                }
                if let Some(tooltip) = &raw.tooltip {
                    permission.tooltip = tooltip.clone();
                }
                if let Some(options) = &raw.options {
                    permission.options = options.clone();
                }
            } else {
                self.permissions.push(Permission {
                    key: raw.key.clone(),
                    name: raw.name.clone().unwrap_or_else(|| raw.key.clone()),
                    options: raw.options.clone().unwrap_or_default(),
                    tooltip: raw.tooltip.clone().unwrap_or_default(),
                });
            }
        }

        if !user.presets.privacy.is_empty() {
            self.privacy_keys = user.presets.privacy.clone();
        }
        if !user.presets.security.is_empty() {
            self.security_keys = user.presets.security.clone();
        }
    }
}

#[allow(dead_code)]
pub const CONTENT_SETTING_KEYS: &[&str] = &[
    "DefaultGeolocationSetting",
    "DefaultVideoCaptureSetting",
    "DefaultAudioCaptureSetting",
    "DefaultNotificationsSetting",
    "DefaultJavaScriptSetting",
    "DefaultImagesSetting",
    "DefaultPopupsSetting",
    "DefaultWebUsbGuardSetting",
    "DefaultSerialGuardSetting",
    "DefaultWebHidGuardSetting",
    "DefaultFileSystemReadGuardSetting",
    "DefaultFileSystemWriteGuardSetting",
    "DefaultClipboardSetting",
    "DefaultWindowPlacementSetting",
    "DefaultLocalFontsSetting",
];
