use std::collections::{BTreeMap, BTreeSet};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

pub const BUILTIN_CATALOG: &str = include_str!("../../assets/catalog.json");

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum JsonValue {
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<String>),
}

impl From<JsonValue> for PolicyValue {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Bool(v) => PolicyValue::Bool(v),
            JsonValue::Int(v) => PolicyValue::Int(v),
            JsonValue::Str(v) => PolicyValue::Str(v),
            JsonValue::Array(v) => PolicyValue::Array(v),
        }
    }
}

#[derive(Deserialize)]
struct RawCatalog {
    #[serde(default)]
    features: Vec<RawFeature>,
    #[serde(default)]
    permissions: Vec<RawPermission>,
    #[serde(default)]
    presets: RawPresets,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RawPresets {
    #[serde(default)]
    pub privacy: Vec<String>,
    #[serde(default)]
    pub security: Vec<String>,
}

#[derive(Deserialize)]
struct RawFeature {
    key: String,
    name: String,
    tooltip: String,
    #[serde(default)]
    value: Option<JsonValue>,
    #[serde(default)]
    windows_value: Option<JsonValue>,
    #[serde(default)]
    privacy: bool,
    #[serde(default)]
    security: bool,
    #[serde(default)]
    section: String,
}

#[derive(Deserialize)]
struct RawPermission {
    key: String,
    name: String,
    tooltip: String,
    #[serde(default)]
    options: Vec<String>,
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
                    name: raw
                        .name
                        .clone()
                        .unwrap_or_else(|| raw.key.clone()),
                    tooltip: raw.tooltip.clone().unwrap_or_default(),
                    value: raw
                        .value
                        .clone()
                        .map(PolicyValue::from)
                        .unwrap_or(PolicyValue::Bool(false)),
                    windows_value: raw.windows_value.clone().map(PolicyValue::from),
                    privacy: raw.privacy.unwrap_or(false),
                    security: raw.security.unwrap_or(false),
                    section: raw
                        .section
                        .clone()
                        .unwrap_or_else(|| "custom".to_owned()),
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UserCatalog {
    #[serde(default)]
    pub features: Vec<UserFeature>,
    #[serde(default)]
    pub permissions: Vec<UserPermission>,
    #[serde(default)]
    pub remove: Vec<String>,
    #[serde(default)]
    pub presets: RawPresets,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserFeature {
    pub key: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub value: Option<JsonValue>,
    #[serde(default)]
    pub windows_value: Option<JsonValue>,
    #[serde(default)]
    pub privacy: Option<bool>,
    #[serde(default)]
    pub security: Option<bool>,
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserPermission {
    pub key: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteCatalog {
    #[serde(default)]
    #[allow(dead_code)]
    pub version: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub source: String,
    #[serde(default)]
    pub policies: BTreeMap<String, RemotePolicy>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemotePolicy {
    #[serde(default)]
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub r#type: String,
}

static CATALOG: OnceLock<RwLock<Catalog>> = OnceLock::new();

pub fn init() -> std::sync::RwLockReadGuard<'static, Catalog> {
    CATALOG
        .get_or_init(|| RwLock::new(build_catalog()))
        .read()
        .expect("catalog lock poisoned")
}

pub fn reload() {
    if let Some(lock) = CATALOG.get() {
        let mut catalog = lock.write().expect("catalog lock poisoned");
        *catalog = build_catalog();
    }
}

fn build_catalog() -> Catalog {
    let mut catalog =
        Catalog::from_json(BUILTIN_CATALOG).expect("builtin catalog must be valid JSON");

    if let Some(user) = load_json_file(user_catalog_path()) {
        match serde_json::from_str::<UserCatalog>(&user) {
            Ok(user) => catalog.merge_user(&user),
            Err(err) => log_merge(&format!("user catalog ignored: {err}")),
        }
    }

    catalog
}

fn load_json_file(path: Option<std::path::PathBuf>) -> Option<String> {
    std::fs::read_to_string(path?).ok()
}

pub fn load_remote(browser: crate::domain::Browser) -> Option<RemoteCatalog> {
    let path = dirs::cache_dir()?.join("slimbrave").join(browser.cache_file());
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

fn user_catalog_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| home.join(".config/slimbrave/catalog.json"))
}

fn log_merge(message: &str) {
    #[cfg(not(test))]
    eprintln!("slimbrave: {message}");
    #[cfg(test)]
    let _ = message;
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

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE: &[(&str, &str, bool, bool)] = &[
        ("MetricsReportingEnabled", "Disable Metrics Reporting", true, true),
        ("SafeBrowsingExtendedReportingEnabled", "Disable Safe Browsing Reporting", true, true),
        ("UrlKeyedAnonymizedDataCollectionEnabled", "Disable URL Data Collection", true, true),
        ("FeedbackSurveysEnabled", "Disable Feedback Surveys", true, true),
        ("BraveP3AEnabled", "Disable P3A Telemetry", true, true),
        ("BraveStatsPingEnabled", "Disable Daily Stats Ping", true, true),
        ("BraveWebDiscoveryEnabled", "Disable Web Discovery", true, true),
        ("AutofillAddressEnabled", "Disable Autofill (Addresses)", true, false),
        ("AutofillCreditCardEnabled", "Disable Autofill (Credit Cards)", true, false),
        ("PasswordManagerEnabled", "Disable Password Manager", true, false),
        ("BrowserSignin", "Disable Browser Sign-in", true, false),
        ("WebRtcIPHandling", "Disable WebRTC IP Leak", true, true),
        ("QuicAllowed", "Disable QUIC Protocol", false, true),
        ("BlockThirdPartyCookies", "Block Third Party Cookies", true, true),
        ("EnableDoNotTrack", "Enable Do Not Track", true, true),
        ("GlobalPrivacyControlEnabled", "Enable Global Privacy Control", true, true),
        ("BraveDeAMPEnabled", "Enable De-AMP", true, true),
        ("BraveDebouncingEnabled", "Enable Debouncing", true, true),
        ("BraveTrackersStrippingEnabled", "Strip URL Trackers", true, true),
        ("ReduceAcceptLanguage", "Reduce Language Fingerprinting", true, true),
        ("ForceGoogleSafeSearch", "Force Google SafeSearch", false, true),
        ("IPFSEnabled", "Disable IPFS", true, true),
        ("IncognitoModeAvailability", "Force Incognito Mode", false, false),
        ("PromptForDownloadLocation", "Force Download Prompts", true, true),
        ("ClearBrowsingDataOnExitList", "Clear Data on Exit", true, false),
        ("HttpsOnlyMode", "Force HTTPS-Only Mode", true, true),
        ("BraveRewardsDisabled", "Disable Brave Rewards", true, true),
        ("BraveWalletDisabled", "Disable Brave Wallet", true, true),
        ("BraveVPNDisabled", "Disable Brave VPN", true, true),
        ("BraveAIChatEnabled", "Disable Brave AI Chat", true, true),
        ("TorDisabled", "Disable Tor", true, true),
        ("SyncDisabled", "Disable Sync", false, true),
        ("BraveNewsDisabled", "Disable Brave News", true, true),
        ("BraveTalkDisabled", "Disable Brave Talk", true, true),
        ("BraveSpeedreaderEnabled", "Disable Speedreader", true, true),
        ("BraveWaybackMachineEnabled", "Disable Wayback Machine Prompts", true, true),
        ("BackgroundModeEnabled", "Disable Background Mode", true, true),
        ("MediaRecommendationsEnabled", "Disable Media Recommendations", true, false),
        ("ShoppingListEnabled", "Disable Shopping List", true, false),
        ("AlwaysOpenPdfExternally", "Always Open PDF Externally", true, true),
        ("TranslateEnabled", "Disable Translate", false, false),
        ("SpellcheckEnabled", "Disable Spellcheck", false, false),
        ("PromotionsEnabled", "Disable Promotions", true, false),
        ("SearchSuggestEnabled", "Disable Search Suggestions", true, false),
        ("PrintingEnabled", "Disable Printing", false, false),
        ("DefaultBrowserSettingEnabled", "Disable Default Browser Prompt", true, false),
        ("DeveloperToolsDisabled", "Disable Developer Tools", false, true),
        ("BravePlaylistEnabled", "Disable Brave Playlist", true, true),
    ];

    #[test]
    fn builtin_catalog_matches_reference() {
        let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid builtin catalog");
        assert_eq!(catalog.features.len(), REFERENCE.len());
        for (expected, actual) in REFERENCE.iter().zip(catalog.features.iter()) {
            assert_eq!(actual.key, expected.0, "key mismatch");
            assert_eq!(actual.name, expected.1, "name mismatch for {}", actual.key);
            assert_eq!(
                (actual.privacy, actual.security),
                (expected.2, expected.3),
                "suggestion flags mismatch for {}",
                actual.key
            );
        }
        assert_eq!(catalog.permissions.len(), 15);
        assert_eq!(catalog.privacy_keys.len(), 40);
        assert_eq!(catalog.security_keys.len(), 29);
    }

    #[test]
    fn catalog_counts_match_reference() {
        let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid builtin catalog");
        let count = |section: &str| {
            catalog
                .features
                .iter()
                .filter(|f| f.section == section)
                .count()
        };
        assert_eq!(count("telemetry"), 7);
        assert_eq!(count("privacy"), 19);
        assert_eq!(count("brave"), 10);
        assert_eq!(count("perf"), 12);
    }

    #[test]
    fn user_layer_overrides_adds_and_removes() {
        let mut catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid");
        let user = UserCatalog {
            features: vec![
                UserFeature {
                    key: "MetricsReportingEnabled".to_owned(),
                    name: Some("My Metric Toggle".to_owned()),
                    tooltip: None,
                    value: None,
                    windows_value: None,
                    privacy: Some(false),
                    security: None,
                    section: None,
                },
                UserFeature {
                    key: "MyCustomPolicy".to_owned(),
                    name: Some("Custom Policy".to_owned()),
                    tooltip: Some("A user-defined policy".to_owned()),
                    value: Some(JsonValue::Bool(true)),
                    windows_value: None,
                    privacy: Some(true),
                    security: Some(true),
                    section: Some("privacy".to_owned()),
                },
            ],
            permissions: vec![],
            remove: vec!["AutofillAddressEnabled".to_owned()],
            presets: RawPresets::default(),
        };
        catalog.merge_user(&user);

        let feature = catalog.feature_by_key("MetricsReportingEnabled").unwrap();
        assert_eq!(feature.name, "My Metric Toggle");
        assert_eq!(feature.tooltip, "Stops Brave from sending anonymous usage and crash reports.");
        assert!(!feature.privacy);

        assert!(catalog.feature_by_key("AutofillAddressEnabled").is_none());
        let custom = catalog.feature_by_key("MyCustomPolicy").unwrap();
        assert_eq!(custom.value, PolicyValue::Bool(true));
        assert_eq!(custom.section, "privacy");
        assert_eq!(catalog.features.len(), 48);
    }

    #[test]
    fn managed_keys_covers_catalog_and_special_keys() {
        let catalog = Catalog::from_json(BUILTIN_CATALOG).expect("valid");
        let keys = catalog.managed_keys();
        assert!(keys.contains("MetricsReportingEnabled"));
        assert!(keys.contains("DefaultGeolocationSetting"));
        assert!(keys.contains("SafeBrowsingProtectionLevel"));
        assert!(keys.contains("DnsOverHttpsTemplates"));
    }
}
