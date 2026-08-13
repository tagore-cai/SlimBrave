use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::domain::{Browser, PlatformKind, RawValue, WriteLevel};
#[cfg(windows)]
use crate::domain::WriteValue;

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


#[cfg(target_os = "macos")]
fn login_name() -> String {
    std::env::var("USER").unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn user_managed_prefs_dir(target_dir: Option<&Path>) -> Option<std::path::PathBuf> {
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

#[cfg(target_os = "macos")]
fn managed_prefs_path(channel: &Channel, target_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    Some(user_managed_prefs_dir(target_dir)?.join(format!("{}.plist", channel.domain)))
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn flush_pref_cache() {
    let _ = std::process::Command::new("killall")
        .args(["cfprefsd"])
        .status();
}

pub fn channels_for(browser: Browser) -> Vec<Channel> {
    match browser {
        Browser::Chrome => CHROME_CHANNELS.to_vec(),
        Browser::Brave => CHANNELS.to_vec(),
    }
}

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

pub fn installed_channels(browser: Browser) -> (Vec<Channel>, bool) {
    let channels = channels_for(browser);
    let found: Vec<Channel> = channels.iter().copied().filter(is_installed).collect();
    if found.is_empty() {
        (vec![channels[0]], false)
    } else {
        (found, true)
    }
}

#[cfg(target_os = "macos")]
fn is_installed(channel: &Channel) -> bool {
    if Path::new(&format!("/Applications/{}.app", channel.app_name)).exists() {
        return true;
    }
    if let Some(home) = dirs::home_dir() {
        if Path::new(&format!("{}/Applications/{}.app", home.display(), channel.app_name)).exists() {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn is_installed(channel: &Channel) -> bool {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return false;
    };
    Path::new(&local).join(channel.data_dir).exists()
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

#[cfg(target_os = "macos")]
fn is_brave_running(channel: &Channel) -> bool {
    let Ok(output) = std::process::Command::new("pgrep")
        .args(["-if", channel.app_name])
        .output()
    else {
        return false;
    };
    !String::from_utf8_lossy(&output.stdout).trim().is_empty()
}

#[cfg(target_os = "macos")]
fn close_brave_process(channel: &Channel) -> bool {
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

#[cfg(target_os = "macos")]
fn remove_managed_pref_files(channel: &Channel, _write_level: WriteLevel) -> Vec<String> {
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
    let system = Path::new("/Library/Managed Preferences").join(format!("{}.plist", channel.domain));
    if system.exists() && std::fs::remove_file(&system).is_ok() {
        removed.push(system.display().to_string());
    }
    flush_pref_cache();
    removed
}

#[cfg(target_os = "macos")]
fn strip_legacy_keys(channel: &Channel) -> Vec<String> {
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

#[cfg(windows)]
fn is_brave_running(_channel: &Channel) -> bool {
    let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq brave.exe", "/FO", "CSV", "/NH"])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).to_ascii_lowercase().contains("brave.exe")
}

#[cfg(windows)]
fn close_brave_process(_channel: &Channel) -> bool {
    let _ = std::process::Command::new("taskkill")
        .args(["/IM", "brave.exe", "/T", "/F"])
        .status();
    !is_brave_running(_channel)
}

#[cfg(windows)]
fn remove_managed_pref_files(channel: &Channel, write_level: WriteLevel) -> Vec<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let mut removed = Vec::new();
    if write_level == WriteLevel::Machine {
        let script = format!(
            "Remove-Item -Path 'HKLM:\\{}' -Recurse -Force -ErrorAction SilentlyContinue",
            channel.reg_path.replace('\\', "\\\\")
        );
        if run_elevated_powershell(&script).is_ok() {
            removed.push(format!("HKLM\\{}", channel.reg_path));
        }
    }
    let root = RegKey::predef(HKEY_CURRENT_USER);
    if root.delete_subkey_all(channel.reg_path).is_ok() {
        removed.push(format!("HKCU\\{}", channel.reg_path));
    }
    removed
}

#[cfg(windows)]
fn strip_legacy_keys(_channel: &Channel) -> Vec<String> {
    Vec::new()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_brave_running(_channel: &Channel) -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn close_brave_process(_channel: &Channel) -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn remove_managed_pref_files(_channel: &Channel, _write_level: WriteLevel) -> Vec<String> {
    Vec::new()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn strip_legacy_keys(_channel: &Channel) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn apply_to(
    channel: &Channel,
    plan: &crate::domain::ApplyPlan,
    dry_run: bool,
    _write_level: WriteLevel,
    target_dir: Option<&Path>,
) -> Result<ApplyReport> {
    let payload: std::collections::BTreeMap<String, crate::domain::WriteValue> =
        plan.write.iter().cloned().collect();

    let (path, removed_profile) = if payload.is_empty() {
        if !dry_run {
            let removed = crate::infrastructure::profile::remove_profile(channel);
            flush_pref_cache();
            (
                "Configuration profile removed".to_owned(),
                Some(removed),
            )
        } else {
            ("(dry run) would remove configuration profile".to_owned(), None)
        }
    } else {
        let path = if dry_run {
            format!(
                "(dry run) would generate SlimBrave-{}.mobileconfig",
                channel.domain
            )
        } else {
            let path = crate::infrastructure::profile::write_mobileconfig(
                channel,
                &payload,
                target_dir,
            )?;
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

#[cfg(target_os = "macos")]
fn read_plist_dict(path: &Path) -> Option<plist::Dictionary> {
    let value = plist::Value::from_file(path).ok()?;
    value.as_dictionary().cloned()
}

#[cfg(windows)]
fn apply_to(
    channel: &Channel,
    plan: &crate::domain::ApplyPlan,
    dry_run: bool,
    write_level: WriteLevel,
    _target_dir: Option<&Path>,
) -> Result<ApplyReport> {
    if write_level == WriteLevel::Machine {
        if dry_run {
            return Ok(ApplyReport {
                channel: channel.name,
                path: format!("(dry run) HKLM\\{}", channel.reg_path),
                written: plan.write.iter().map(|(key, _)| key.clone()).collect(),
                deleted: plan.delete.clone(),
                dry_run,
                removed_profile: None,
            });
        }
        let script = build_machine_registry_script(channel, plan);
        run_elevated_powershell(&script)?;
        return Ok(ApplyReport {
            channel: channel.name,
            path: format!("HKLM\\{}", channel.reg_path),
            written: plan.write.iter().map(|(key, _)| key.clone()).collect(),
            deleted: plan.delete.clone(),
            dry_run,
            removed_profile: None,
        });
    }

    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = root.create_subkey(channel.reg_path)?;

    if !dry_run {
        for name in &plan.delete {
            let _ = key.delete_value(name);
        }
        for (name, value) in &plan.write {
            match value {
                WriteValue::Bool(value) => key.set_value(name, &u32::from(*value))?,
                WriteValue::Int(value) => key.set_value(name, &(*value as u32))?,
                WriteValue::Str(value) => key.set_value(name, value)?,
                WriteValue::Array(items) => key.set_value(name, items)?,
            }
        }
    }

    Ok(ApplyReport {
        channel: channel.name,
        path: format!("HKCU\\{}", channel.reg_path),
        written: plan.write.iter().map(|(key, _)| key.clone()).collect(),
        deleted: plan.delete.clone(),
        dry_run,
        removed_profile: None,
    })
}

#[cfg(windows)]
fn build_machine_registry_script(
    channel: &Channel,
    plan: &crate::domain::ApplyPlan,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "$p = 'HKLM:\\{}'",
        channel.reg_path.replace('\\', "\\\\")
    ));
    lines.push("New-Item -Path $p -Force | Out-Null".to_owned());
    for name in &plan.delete {
        lines.push(format!(
            "Remove-ItemProperty -Path $p -Name '{}' -ErrorAction SilentlyContinue",
            name.replace('\'', "''")
        ));
    }
    for (name, value) in &plan.write {
        let value_expr = match value {
            WriteValue::Bool(value) => format!("[int]{}", if *value { 1 } else { 0 }),
            WriteValue::Int(value) => value.to_string(),
            WriteValue::Str(value) => {
                format!("'{}'", value.replace('\'', "''"))
            }
            WriteValue::Array(items) => {
                let joined: Vec<String> = items
                    .iter()
                    .map(|item| format!("'{}'", item.replace('\'', "''")))
                    .collect();
                format!("@({})", joined.join(","))
            }
        };
        let reg_type = match value {
            WriteValue::Bool(_) | WriteValue::Int(_) => "DWord",
            WriteValue::Str(_) => "String",
            WriteValue::Array(_) => "MultiString",
        };
        lines.push(format!(
            "Set-ItemProperty -Path $p -Name '{}' -Value {} -Type {}",
            name.replace('\'', "''"),
            value_expr,
            reg_type
        ));
    }
    lines.join("\r\n")
}

#[cfg(windows)]
fn run_elevated_powershell(script: &str) -> Result<()> {
    use anyhow::Context;
    use base64::Engine;

    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>(),
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList '-NoProfile','-EncodedCommand','{encoded}'"
            ),
        ])
        .status()
        .context("could not elevate for HKLM write")?;
    Ok(())
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

#[cfg(target_os = "macos")]
fn read_defaults_domain(channel: &Channel) -> Option<plist::Dictionary> {
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
#[cfg(target_os = "macos")]
fn read_from(
    channel: &Channel,
    target_dir: Option<&Path>,
) -> Result<BTreeMap<String, RawValue>> {
    let Some(user_path) = managed_prefs_path(channel, target_dir) else {
        return Ok(BTreeMap::new());
    };
    if let Some(dict) = read_plist_dict(&user_path) {
        return Ok(dict_to_raw(dict));
    }

    if target_dir.is_none() {
        let system_path = Path::new("/Library/Managed Preferences")
            .join(format!("{}.plist", channel.domain));
        if let Some(dict) = read_plist_dict(&system_path) {
            return Ok(dict_to_raw(dict));
        }
    }

    Ok(BTreeMap::new())
}

#[cfg(target_os = "macos")]
fn dict_to_raw(dict: plist::Dictionary) -> BTreeMap<String, RawValue> {
    let mut raw = BTreeMap::new();
    for (key, value) in dict {
        if let Some(raw_value) = from_plist_value(&value) {
            raw.insert(key, raw_value);
        }
    }
    raw
}

#[cfg(target_os = "macos")]
fn from_plist_value(value: &plist::Value) -> Option<RawValue> {
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

#[cfg(windows)]
fn read_from(
    channel: &Channel,
    _target_dir: Option<&Path>,
) -> Result<BTreeMap<String, RawValue>> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let mut raw = BTreeMap::new();

    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(channel.reg_path) {
        read_registry_values(&key, &mut raw)?;
    }

    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(channel.reg_path) {
        read_registry_values(&key, &mut raw)?;
    }
    Ok(raw)
}

#[cfg(windows)]
fn read_registry_values(
    key: &winreg::RegKey,
    raw: &mut BTreeMap<String, RawValue>,
) -> Result<()> {
    for item in key.enum_values() {
        let (name, value) = item?;
        let winreg::RegValue { bytes, vtype } = value;
        let raw_value = match vtype {
            winreg::enums::RegType::REG_DWORD | winreg::enums::RegType::REG_QWORD => {
                let int = if bytes.len() >= 4 {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64
                } else {
                    continue;
                };
                RawValue::Int(int)
            }
            winreg::enums::RegType::REG_SZ | winreg::enums::RegType::REG_EXPAND_SZ => {
                RawValue::Str(
                    String::from_utf8_lossy(&bytes)
                        .trim_end_matches('\0')
                        .to_string(),
                )
            }
            winreg::enums::RegType::REG_MULTI_SZ => {
                let items: Vec<String> = bytes
                    .split(|byte| *byte == 0)
                    .filter(|chunk| !chunk.is_empty())
                    .map(|chunk| String::from_utf8_lossy(chunk).to_string())
                    .collect();
                RawValue::Array(items)
            }
            _ => continue,
        };
        raw.entry(name).or_insert(raw_value);
    }
    Ok(())
}

#[cfg(test)]
mod machine_tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn machine_script_contains_writes_and_deletes() {
        let plan = crate::domain::ApplyPlan {
            write: vec![
                ("BraveRewardsDisabled".to_owned(), WriteValue::Bool(true)),
                ("BrowserSignin".to_owned(), WriteValue::Int(0)),
                ("DnsOverHttpsMode".to_owned(), WriteValue::Str("off".to_owned())),
                ("ClearBrowsingDataOnExitList".to_owned(), WriteValue::Array(vec!["a".to_owned(), "b".to_owned()])),
            ],
            delete: vec!["AutofillAddressEnabled".to_owned()],
        };
        let script = build_machine_registry_script(&CHANNELS[0], &plan);
        assert!(script.contains("HKLM:\\Software\\Policies\\BraveSoftware\\Brave"));
        assert!(script.contains("Set-ItemProperty -Path $p -Name 'BraveRewardsDisabled' -Value [int]1 -Type DWord"));
        assert!(script.contains("Set-ItemProperty -Path $p -Name 'BrowserSignin' -Value 0 -Type DWord"));
        assert!(script.contains("Set-ItemProperty -Path $p -Name 'DnsOverHttpsMode' -Value 'off' -Type String"));
        assert!(script.contains("Set-ItemProperty -Path $p -Name 'ClearBrowsingDataOnExitList' -Value @('a','b') -Type MultiString"));
        assert!(script.contains("Remove-ItemProperty -Path $p -Name 'AutofillAddressEnabled'"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApplyPlan, WriteValue};

    fn plan() -> ApplyPlan {
        ApplyPlan {
            write: vec![
                ("MetricsReportingEnabled".to_string(), WriteValue::Bool(false)),
                ("BrowserSignin".to_string(), WriteValue::Int(0)),
                ("DnsOverHttpsMode".to_string(), WriteValue::Str("off".to_string())),
            ],
            delete: vec!["AutofillAddressEnabled".to_string()],
        }
    }

    #[test]
    fn detect_returns_a_platform() {
        let platform = detect();
        assert!(matches!(platform, PlatformKind::MacOs | PlatformKind::Windows));
    }

    #[test]
    fn installed_channels_never_empty() {
        let (channels, _) = installed_channels(Browser::Brave);
        assert!(!channels.is_empty());
        assert!(channels.iter().all(|c| CHANNELS.contains(c)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_apply_writes_mobileconfig() {
        let temp = tempfile::tempdir().unwrap();
        let plan = plan();
        let report = apply_to(&CHANNELS[0], &plan, false, WriteLevel::User, Some(temp.path())).unwrap();

        assert!(!report.dry_run);
        assert_eq!(report.written, vec!["MetricsReportingEnabled", "BrowserSignin", "DnsOverHttpsMode"]);
        assert_eq!(report.deleted, vec!["AutofillAddressEnabled"]);
        assert!(report.path.ends_with("SlimBrave-com.brave.Browser.mobileconfig"));

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
        let mcx = domain
            .get("Forced")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .as_dictionary()
            .unwrap()
            .get("mcx_preference_settings")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(
            mcx.get("MetricsReportingEnabled").unwrap().as_boolean().unwrap(),
            false
        );
        assert_eq!(mcx.get("BrowserSignin").unwrap().as_signed_integer().unwrap(), 0);
        assert_eq!(mcx.get("DnsOverHttpsMode").unwrap().as_string().unwrap(), "off");
        assert!(!mcx.contains_key("AutofillAddressEnabled"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_dry_run_does_not_write() {
        let temp = tempfile::tempdir().unwrap();
        let plan = plan();
        let report = apply_to(&CHANNELS[0], &plan, true, WriteLevel::User, Some(temp.path())).unwrap();

        assert!(report.dry_run);
        assert!(!temp.path().join("SlimBrave-com.brave.Browser.mobileconfig").exists());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_empty_plan_removes_profile() {
        let temp = tempfile::tempdir().unwrap();
        let empty_plan = ApplyPlan {
            write: vec![],
            delete: vec!["MetricsReportingEnabled".to_owned()],
        };
        let report = apply_to(&CHANNELS[0], &empty_plan, false, WriteLevel::User, Some(temp.path())).unwrap();
        assert_eq!(report.removed_profile, Some(false));
        assert!(!temp.path().join("SlimBrave-com.brave.Browser.mobileconfig").exists());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_read_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join(login_name());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("com.brave.Browser.plist");
        let mut dict = plist::Dictionary::new();
        dict.insert("MetricsReportingEnabled".to_owned(), plist::Value::Boolean(false));
        dict.insert("BrowserSignin".to_owned(), plist::Value::Integer(0.into()));
        dict.insert("DnsOverHttpsMode".to_owned(), plist::Value::String("off".to_owned()));
        plist::Value::Dictionary(dict).to_file_xml(&path).unwrap();

        let raw = read_from(&CHANNELS[0], Some(temp.path())).unwrap();
        assert_eq!(raw.get("MetricsReportingEnabled"), Some(&RawValue::Bool(false)));
        assert_eq!(raw.get("BrowserSignin"), Some(&RawValue::Int(0)));
        assert_eq!(raw.get("DnsOverHttpsMode"), Some(&RawValue::Str("off".to_string())));
    }
}
