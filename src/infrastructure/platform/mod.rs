//! 平台适配门面：渠道目录、进程管理、策略读写。
//!
//! 平台相关实现隔离在 [`macos`] / [`windows`] / [`stub`]，本模块只暴露跨平台 API：
//! - 渠道数据：[`Channel`]、[`CHANNELS`]、[`channels_for`]、[`installed_channels`]
//! - 进程：[`brave_is_running`]、[`close_brave`]
//! - 读写：[`apply`]、[`read`]、[`merged_policy_source`]、[`remove_managed_prefs`]、[`strip_legacy_domain_keys`]

use std::collections::BTreeMap;

use anyhow::Result;

use crate::domain::{Browser, PlatformKind, RawValue, WriteLevel};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::has_legacy_plists;
#[cfg(target_os = "macos")]
use macos::{
    apply_to, close_brave_process, is_brave_running, is_installed, read_defaults_domain, read_from,
    remove_managed_pref_files, strip_legacy_keys,
};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::{
    apply_to, close_brave_process, is_brave_running, is_installed, read_from,
    remove_managed_pref_files, strip_legacy_keys,
};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod stub;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use stub::{
    apply_to, close_brave_process, is_brave_running, is_installed, read_from,
    remove_managed_pref_files, strip_legacy_keys,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Channel {
    pub name: &'static str,
    pub domain: &'static str,
    #[allow(dead_code)]
    pub app_name: &'static str,
    #[allow(dead_code)]
    pub data_dir: &'static str,
    #[allow(dead_code)]
    pub reg_path: &'static str,
}

pub const CHANNELS: [Channel; 4] = [
    Channel {
        name: "Release",
        domain: "com.brave.Browser",
        app_name: "Brave Browser",
        data_dir: "BraveSoftware/Brave-Browser",
        reg_path: "Software\\Policies\\BraveSoftware\\Brave",
    },
    Channel {
        name: "Beta",
        domain: "com.brave.Browser.beta",
        app_name: "Brave Browser Beta",
        data_dir: "BraveSoftware/Brave-Browser-Beta",
        reg_path: "Software\\Policies\\BraveSoftware\\BraveBeta",
    },
    Channel {
        name: "Dev",
        domain: "com.brave.Browser.dev",
        app_name: "Brave Browser Dev",
        data_dir: "BraveSoftware/Brave-Browser-Dev",
        reg_path: "Software\\Policies\\BraveSoftware\\BraveDev",
    },
    Channel {
        name: "Nightly",
        domain: "com.brave.Browser.nightly",
        app_name: "Brave Browser Nightly",
        data_dir: "BraveSoftware/Brave-Browser-Nightly",
        reg_path: "Software\\Policies\\BraveSoftware\\BraveNightly",
    },
];

const CHROME_CHANNELS: [Channel; 4] = [
    Channel {
        name: "Stable",
        domain: "com.google.Chrome",
        app_name: "Google Chrome",
        data_dir: "Google/Chrome",
        reg_path: "Software\\Policies\\Google\\Chrome",
    },
    Channel {
        name: "Beta",
        domain: "com.google.Chrome.beta",
        app_name: "Google Chrome Beta",
        data_dir: "Google/Chrome Beta",
        reg_path: "Software\\Policies\\Google\\ChromeBeta",
    },
    Channel {
        name: "Dev",
        domain: "com.google.Chrome.dev",
        app_name: "Google Chrome Dev",
        data_dir: "Google/Chrome Dev",
        reg_path: "Software\\Policies\\Google\\ChromeDev",
    },
    Channel {
        name: "Canary",
        domain: "com.google.Chrome.canary",
        app_name: "Google Chrome Canary",
        data_dir: "Google/Chrome Canary",
        reg_path: "Software\\Policies\\Google\\ChromeCanary",
    },
];

#[derive(Debug, Clone)]
pub struct ApplyReport {
    pub channel: &'static str,
    pub path: String,
    pub written: Vec<String>,
    pub deleted: Vec<String>,
    pub dry_run: bool,
    pub removed_profile: Option<bool>,
}

pub fn detect() -> PlatformKind {
    if cfg!(target_os = "macos") {
        PlatformKind::MacOs
    } else if cfg!(target_os = "windows") {
        PlatformKind::Windows
    } else {
        panic!("SlimBrave currently supports macOS and Windows only")
    }
}

pub fn channels_for(browser: Browser) -> Vec<Channel> {
    match browser {
        Browser::Chrome => CHROME_CHANNELS.to_vec(),
        Browser::Brave => CHANNELS.to_vec(),
    }
}

pub fn installed_channels(browser: Browser) -> (Vec<Channel>, bool) {
    let channels = channels_for(browser);
    let found: Vec<Channel> = channels.iter().copied().filter(is_installed).collect();
    if found.is_empty() {
        (vec![channels[0]], false)
    } else {
        (found, true)
    }
}

pub fn apply(
    channel: &Channel,
    plan: &crate::domain::ApplyPlan,
    dry_run: bool,
    write_level: WriteLevel,
) -> Result<ApplyReport> {
    apply_to(channel, plan, dry_run, write_level, None)
}

pub fn brave_is_running(channel: &Channel) -> bool {
    is_brave_running(channel)
}

pub fn close_brave(channel: &Channel) -> bool {
    close_brave_process(channel)
}

pub fn remove_managed_prefs(channel: &Channel, write_level: WriteLevel) -> Vec<String> {
    remove_managed_pref_files(channel, write_level)
}

pub fn strip_legacy_domain_keys(channel: &Channel) -> Vec<String> {
    strip_legacy_keys(channel)
}

pub fn read(channel: &Channel) -> Result<BTreeMap<String, RawValue>> {
    read_from(channel, None)
}

pub fn merged_policy_source(channel: &Channel) -> BTreeMap<String, RawValue> {
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut payload = read(channel).unwrap_or_default();
    #[cfg(target_os = "macos")]
    {
        if let Some(legacy) = read_defaults_domain(channel) {
            for key in crate::domain::catalog::init().managed_keys() {
                if let Some(value) = legacy.get(key.as_str()) {
                    if let Some(raw_value) = from_plist_value(value) {
                        payload.entry(key.clone()).or_insert(raw_value);
                    }
                }
            }
        }
    }
    payload
}

/// plist 值 → [`RawValue`]，由门面（合并 legacy 域）与 macOS 读路径共用。
#[cfg(target_os = "macos")]
pub(super) fn from_plist_value(value: &plist::Value) -> Option<RawValue> {
    match value {
        plist::Value::Boolean(value) => Some(RawValue::Bool(*value)),
        plist::Value::Integer(value) => Some(RawValue::Int(value.as_signed().unwrap_or(0))),
        plist::Value::String(value) => Some(RawValue::Str(value.clone())),
        plist::Value::Array(items) => {
            let strings = items
                .iter()
                .map(|item| item.as_string().map(str::to_string))
                .collect::<Option<Vec<String>>>();
            strings.map(RawValue::Array)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_a_platform() {
        let platform = detect();
        assert!(matches!(
            platform,
            PlatformKind::MacOs | PlatformKind::Windows
        ));
    }

    #[test]
    fn installed_channels_never_empty() {
        let (channels, _) = installed_channels(Browser::Brave);
        assert!(!channels.is_empty());
        assert!(channels.iter().all(|c| CHANNELS.contains(c)));
    }
}
