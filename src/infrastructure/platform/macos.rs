//! macOS 平台实现：Managed Preferences plist 读写、配置描述文件、进程管理。

use std::path::{Path, PathBuf};

use anyhow::Result;

#[cfg(test)]
use super::CHANNELS;
use super::{from_plist_value, ApplyReport, Channel};

fn login_name() -> String {
    std::env::var("USER").unwrap_or_default()
}

fn user_managed_prefs_dir(target_dir: Option<&Path>) -> Option<PathBuf> {
    match target_dir {
        Some(path) => Some(path.join(login_name())),
        None => {
            let home = dirs::home_dir()?;
            let login = login_name();
            if login.is_empty() {
                Some(home.join("Library/Managed Preferences"))
            } else {
                Some(home.join("Library/Managed Preferences").join(login))
            }
        }
    }
}

fn managed_prefs_path(channel: &Channel, target_dir: Option<&Path>) -> Option<PathBuf> {
    Some(user_managed_prefs_dir(target_dir)?.join(format!("{}.plist", channel.domain)))
}

pub(crate) fn has_legacy_plists(channel: &Channel) -> bool {
    if let Some(user) = managed_prefs_path(channel, None) {
        if user.exists() {
            return true;
        }
    }
    if let Some(home) = dirs::home_dir() {
        let legacy = home
            .join("Library/Managed Preferences")
            .join(format!("{}.plist", channel.domain));
        if legacy.exists() {
            return true;
        }
    }
    Path::new("/Library/Managed Preferences")
        .join(format!("{}.plist", channel.domain))
        .exists()
}

fn flush_pref_cache() {
    let _ = std::process::Command::new("killall")
        .args(["cfprefsd"])
        .status();
}

pub(super) fn is_installed(channel: &Channel) -> bool {
    if Path::new(&format!("/Applications/{}.app", channel.app_name)).exists() {
        return true;
    }
    if let Some(home) = dirs::home_dir() {
        if Path::new(&format!(
            "{}/Applications/{}.app",
            home.display(),
            channel.app_name
        ))
        .exists()
        {
            return true;
        }
    }
    false
}

pub(super) fn is_brave_running(channel: &Channel) -> bool {
    let Ok(output) = std::process::Command::new("pgrep")
        .args(["-if", channel.app_name])
        .output()
    else {
        return false;
    };
    !String::from_utf8_lossy(&output.stdout).trim().is_empty()
}

pub(super) fn close_brave_process(channel: &Channel) -> bool {
    use std::time::Duration;

    let app = channel.app_name;
    let _ = std::process::Command::new("osascript")
        .args(["-e", &format!("tell application \"{app}\" to quit")])
        .status();
    std::thread::sleep(Duration::from_millis(2000));
    let _ = std::process::Command::new("pkill")
        .args(["-TERM", "-if", app])
        .status();
    std::thread::sleep(Duration::from_millis(2000));

    if is_brave_running(channel) {
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "-if", app])
            .status();
        std::thread::sleep(Duration::from_millis(1500));
    }
    !is_brave_running(channel)
}

pub(super) fn remove_managed_pref_files(
    channel: &Channel,
    _write_level: crate::domain::WriteLevel,
) -> Vec<String> {
    let mut removed = Vec::new();
    if let Some(user) = managed_prefs_path(channel, None) {
        if user.exists() && std::fs::remove_file(&user).is_ok() {
            removed.push(user.display().to_string());
        }
    }
    if let Some(home) = dirs::home_dir() {
        let legacy = home
            .join("Library/Managed Preferences")
            .join(format!("{}.plist", channel.domain));
        if legacy.exists() && std::fs::remove_file(&legacy).is_ok() {
            removed.push(legacy.display().to_string());
        }
    }
    let system =
        Path::new("/Library/Managed Preferences").join(format!("{}.plist", channel.domain));
    if system.exists() && std::fs::remove_file(&system).is_ok() {
        removed.push(system.display().to_string());
    }
    flush_pref_cache();
    removed
}

pub(super) fn strip_legacy_keys(channel: &Channel) -> Vec<String> {
    let Some(domain_data) = read_defaults_domain(channel) else {
        return Vec::new();
    };
    let mut removed = Vec::new();
    for key in crate::domain::catalog::init().managed_keys() {
        if domain_data.contains_key(key.as_str()) {
            let status = std::process::Command::new("defaults")
                .args(["delete", channel.domain, key.as_str()])
                .status();
            if matches!(status, Ok(status) if status.success()) {
                removed.push(key.clone());
            }
        }
    }
    removed
}

pub(super) fn apply_to(
    channel: &Channel,
    plan: &crate::domain::ApplyPlan,
    dry_run: bool,
    _write_level: crate::domain::WriteLevel,
    target_dir: Option<&Path>,
) -> Result<ApplyReport> {
    let payload: std::collections::BTreeMap<String, crate::domain::WriteValue> =
        plan.write.iter().cloned().collect();

    let (path, removed_profile) = if payload.is_empty() {
        if !dry_run {
            let removed = crate::infrastructure::profile::remove_profile(channel);
            flush_pref_cache();
            ("Configuration profile removed".to_owned(), Some(removed))
        } else {
            (
                "(dry run) would remove configuration profile".to_owned(),
                None,
            )
        }
    } else {
        let path = if dry_run {
            format!(
                "(dry run) would generate SlimBrave-{}.mobileconfig",
                channel.domain
            )
        } else {
            let path =
                crate::infrastructure::profile::write_mobileconfig(channel, &payload, target_dir)?;
            crate::infrastructure::profile::open_config(&path)?;
            flush_pref_cache();
            path.display().to_string()
        };
        (path, None)
    };

    Ok(ApplyReport {
        channel: channel.name,
        path,
        written: plan.write.iter().map(|(key, _)| key.clone()).collect(),
        deleted: plan.delete.clone(),
        dry_run,
        removed_profile,
    })
}

fn read_plist_dict(path: &Path) -> Option<plist::Dictionary> {
    let value = plist::Value::from_file(path).ok()?;
    value.as_dictionary().cloned()
}

pub(super) fn read_defaults_domain(channel: &Channel) -> Option<plist::Dictionary> {
    let Ok(output) = std::process::Command::new("defaults")
        .args(["export", channel.domain, "-"])
        .output()
    else {
        return None;
    };
    if !output.status.success() {
        return None;
    }
    let mut cursor = std::io::Cursor::new(output.stdout);
    match plist::Value::from_reader(&mut cursor) {
        Ok(plist::Value::Dictionary(dict)) => Some(dict),
        _ => None,
    }
}

pub(super) fn read_from(
    channel: &Channel,
    target_dir: Option<&Path>,
) -> Result<std::collections::BTreeMap<String, crate::domain::RawValue>> {
    use std::collections::BTreeMap;

    let Some(user_path) = managed_prefs_path(channel, target_dir) else {
        return Ok(BTreeMap::new());
    };
    if let Some(dict) = read_plist_dict(&user_path) {
        return Ok(dict_to_raw(dict));
    }

    if target_dir.is_none() {
        let system_path =
            Path::new("/Library/Managed Preferences").join(format!("{}.plist", channel.domain));
        if let Some(dict) = read_plist_dict(&system_path) {
            return Ok(dict_to_raw(dict));
        }
    }

    Ok(BTreeMap::new())
}

fn dict_to_raw(
    dict: plist::Dictionary,
) -> std::collections::BTreeMap<String, crate::domain::RawValue> {
    let mut raw = std::collections::BTreeMap::new();
    for (key, value) in dict {
        if let Some(raw_value) = from_plist_value(&value) {
            raw.insert(key, raw_value);
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApplyPlan, WriteLevel, WriteValue};

    fn plan() -> ApplyPlan {
        ApplyPlan {
            write: vec![
                (
                    "MetricsReportingEnabled".to_string(),
                    WriteValue::Bool(false),
                ),
                ("BrowserSignin".to_string(), WriteValue::Int(0)),
                (
                    "DnsOverHttpsMode".to_string(),
                    WriteValue::Str("off".to_string()),
                ),
            ],
            delete: vec!["AutofillAddressEnabled".to_string()],
        }
    }

    #[test]
    fn macos_apply_writes_mobileconfig() {
        let temp = tempfile::tempdir().unwrap();
        let plan = plan();
        let report = apply_to(
            &CHANNELS[0],
            &plan,
            false,
            WriteLevel::User,
            Some(temp.path()),
        )
        .unwrap();

        assert!(!report.dry_run);
        assert_eq!(
            report.written,
            vec![
                "MetricsReportingEnabled",
                "BrowserSignin",
                "DnsOverHttpsMode"
            ]
        );
        assert_eq!(report.deleted, vec!["AutofillAddressEnabled"]);
        assert!(report
            .path
            .ends_with("SlimBrave-com.brave.Browser.mobileconfig"));

        let path = temp.path().join("SlimBrave-com.brave.Browser.mobileconfig");
        let value = plist::Value::from_file(&path).unwrap();
        let dict = value.as_dictionary().unwrap();
        let content = dict.get("PayloadContent").unwrap().as_array().unwrap();
        let main = content[0].as_dictionary().unwrap();
        let domain = main
            .get("PayloadContent")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("com.brave.Browser")
            .unwrap()
            .as_dictionary()
            .unwrap();
        let mcx = domain.get("Forced").unwrap().as_array().unwrap()[0]
            .as_dictionary()
            .unwrap()
            .get("mcx_preference_settings")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(
            mcx.get("MetricsReportingEnabled")
                .unwrap()
                .as_boolean()
                .unwrap(),
            false
        );
        assert_eq!(
            mcx.get("BrowserSignin")
                .unwrap()
                .as_signed_integer()
                .unwrap(),
            0
        );
        assert_eq!(
            mcx.get("DnsOverHttpsMode").unwrap().as_string().unwrap(),
            "off"
        );
        assert!(!mcx.contains_key("AutofillAddressEnabled"));
    }

    #[test]
    fn macos_dry_run_does_not_write() {
        let temp = tempfile::tempdir().unwrap();
        let plan = plan();
        let report = apply_to(
            &CHANNELS[0],
            &plan,
            true,
            WriteLevel::User,
            Some(temp.path()),
        )
        .unwrap();

        assert!(report.dry_run);
        assert!(!temp
            .path()
            .join("SlimBrave-com.brave.Browser.mobileconfig")
            .exists());
    }

    #[test]
    fn macos_empty_plan_removes_profile() {
        let temp = tempfile::tempdir().unwrap();
        let empty_plan = ApplyPlan {
            write: vec![],
            delete: vec!["MetricsReportingEnabled".to_owned()],
        };
        let report = apply_to(
            &CHANNELS[0],
            &empty_plan,
            false,
            WriteLevel::User,
            Some(temp.path()),
        )
        .unwrap();
        assert_eq!(report.removed_profile, Some(false));
        assert!(!temp
            .path()
            .join("SlimBrave-com.brave.Browser.mobileconfig")
            .exists());
    }

    #[test]
    fn macos_read_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join(login_name());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("com.brave.Browser.plist");
        let mut dict = plist::Dictionary::new();
        dict.insert(
            "MetricsReportingEnabled".to_owned(),
            plist::Value::Boolean(false),
        );
        dict.insert("BrowserSignin".to_owned(), plist::Value::Integer(0.into()));
        dict.insert(
            "DnsOverHttpsMode".to_owned(),
            plist::Value::String("off".to_owned()),
        );
        plist::Value::Dictionary(dict).to_file_xml(&path).unwrap();

        let raw = read_from(&CHANNELS[0], Some(temp.path())).unwrap();
        assert_eq!(
            raw.get("MetricsReportingEnabled"),
            Some(&crate::domain::RawValue::Bool(false))
        );
        assert_eq!(
            raw.get("BrowserSignin"),
            Some(&crate::domain::RawValue::Int(0))
        );
        assert_eq!(
            raw.get("DnsOverHttpsMode"),
            Some(&crate::domain::RawValue::Str("off".to_string()))
        );
    }
}
