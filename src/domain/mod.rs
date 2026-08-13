pub mod catalog;
pub mod payload;
pub mod state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteLevel {
    User,
    Machine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    MacOs,
    Windows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Brave,
}

impl Browser {
    pub fn cache_file(self) -> &'static str {
        match self {
            Browser::Chrome => "catalog.remote-chrome.json",
            Browser::Brave => "catalog.remote-brave.json",
        }
    }
}

pub use catalog::{init as catalog};
pub use payload::{
    apply_payload_to_ui, build_apply_plan, sanitize_payload, ApplyPlan, RawValue, WriteValue,
};
pub use state::{DNS_MODE_OPTIONS, Preset, SAFE_BROWSING_OPTIONS, StateSnapshot, UiState};

