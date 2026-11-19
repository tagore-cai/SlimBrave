//! 目录的 JSON 序列化结构：内置清单、用户覆盖层、远程模板层。
//!
//! 这些类型只负责 serde 数据塑形，领域行为见 [`super::model`]。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::PolicyValue;

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
pub(crate) struct RawCatalog {
    #[serde(default)]
    pub(crate) features: Vec<RawFeature>,
    #[serde(default)]
    pub(crate) permissions: Vec<RawPermission>,
    #[serde(default)]
    pub(crate) presets: RawPresets,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RawPresets {
    #[serde(default)]
    pub privacy: Vec<String>,
    #[serde(default)]
    pub security: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct RawFeature {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) tooltip: String,
    #[serde(default)]
    pub(crate) value: Option<JsonValue>,
    #[serde(default)]
    pub(crate) windows_value: Option<JsonValue>,
    #[serde(default)]
    pub(crate) privacy: bool,
    #[serde(default)]
    pub(crate) security: bool,
    #[serde(default)]
    pub(crate) section: String,
}

#[derive(Deserialize)]
pub(crate) struct RawPermission {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) tooltip: String,
    #[serde(default)]
    pub(crate) options: Vec<String>,
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
