#[cfg(not(target_os = "windows"))]
pub mod fetch;
pub mod i18n;
pub mod platform;
#[cfg(target_os = "macos")]
pub mod profile;
