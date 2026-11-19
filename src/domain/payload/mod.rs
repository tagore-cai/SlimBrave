//! 策略载荷：把 [`UiState`](super::UiState) 生成写入计划、清洗外部载荷、回灌 UI。
//!
//! 三个用例彼此独立，分别位于 [`plan`]、[`sanitize`]、[`restore`]；
//! 本模块保存共享的值类型与转换 / 查找助手。

mod plan;
mod restore;
mod sanitize;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::domain::catalog::PolicyValue;
use crate::domain::PlatformKind;

pub use plan::build_apply_plan;
pub use restore::apply_payload_to_ui;
pub use sanitize::sanitize_payload;

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

pub(super) fn expected_value(
    feature: &crate::domain::catalog::Feature,
    platform: PlatformKind,
) -> PolicyValue {
    match platform {
        PlatformKind::Windows => feature
            .windows_value
            .as_ref()
            .unwrap_or(&feature.value)
            .clone(),
        PlatformKind::MacOs => feature.value.clone(),
    }
}

pub(super) fn feature_index(key: &str) -> Option<usize> {
    crate::domain::catalog::init()
        .features
        .iter()
        .position(|feature| feature.key == key)
}

pub(super) fn permission_index(key: &str) -> Option<usize> {
    crate::domain::catalog::init()
        .permissions
        .iter()
        .position(|permission| permission.key == key)
}
