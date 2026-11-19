//! UI 状态、序列化快照与快捷预设。
//!
//! - [`ui_state`]：内存中的勾选 / 选择状态容器
//! - [`snapshot`]：与 JSON 导入导出格式互转
//! - [`preset`]：高隐私 / 高安全预设应用

mod preset;
mod snapshot;
mod ui_state;

#[cfg(test)]
mod tests;

pub use preset::Preset;
pub use snapshot::StateSnapshot;
pub use ui_state::UiState;

pub const NOT_SET: &str = "Not Set";
pub const SAFE_BROWSING_OPTIONS: [&str; 2] = ["On", "Off"];
pub const DNS_MODE_OPTIONS: [&str; 4] = ["Automatic", "Off", "Secure", "Custom"];
