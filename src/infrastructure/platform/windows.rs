//! Windows 平台实现：注册表读写、提权写入 HKLM、进程管理。

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::domain::{RawValue, WriteLevel, WriteValue};

#[cfg(test)]
use super::CHANNELS;
use super::{ApplyReport, Channel};

pub(super) fn is_installed(channel: &Channel) -> bool {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return false;
    };
    Path::new(&local).join(channel.data_dir).exists()
}

pub(super) fn is_brave_running(_channel: &Channel) -> bool {
    let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq brave.exe", "/FO", "CSV", "/NH"])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout)
        .to_ascii_lowercase()
        .contains("brave.exe")
}

pub(super) fn close_brave_process(_channel: &Channel) -> bool {
    let _ = std::process::Command::new("taskkill")
        .args(["/IM", "brave.exe", "/T", "/F"])
        .status();
    !is_brave_running(_channel)
}

pub(super) fn remove_managed_pref_files(channel: &Channel, write_level: WriteLevel) -> Vec<String> {
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

pub(super) fn strip_legacy_keys(_channel: &Channel) -> Vec<String> {
    Vec::new()
}

pub(super) fn apply_to(
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

fn build_machine_registry_script(channel: &Channel, plan: &crate::domain::ApplyPlan) -> String {
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

pub(super) fn read_from(
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

fn read_registry_values(key: &winreg::RegKey, raw: &mut BTreeMap<String, RawValue>) -> Result<()> {
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
    fn machine_script_contains_writes_and_deletes() {
        let plan = crate::domain::ApplyPlan {
            write: vec![
                ("BraveRewardsDisabled".to_owned(), WriteValue::Bool(true)),
                ("BrowserSignin".to_owned(), WriteValue::Int(0)),
                (
                    "DnsOverHttpsMode".to_owned(),
                    WriteValue::Str("off".to_owned()),
                ),
                (
                    "ClearBrowsingDataOnExitList".to_owned(),
                    WriteValue::Array(vec!["a".to_owned(), "b".to_owned()]),
                ),
            ],
            delete: vec!["AutofillAddressEnabled".to_owned()],
        };
        let script = build_machine_registry_script(&CHANNELS[0], &plan);
        assert!(script.contains("HKLM:\\Software\\Policies\\BraveSoftware\\Brave"));
        assert!(script.contains(
            "Set-ItemProperty -Path $p -Name 'BraveRewardsDisabled' -Value [int]1 -Type DWord"
        ));
        assert!(
            script.contains("Set-ItemProperty -Path $p -Name 'BrowserSignin' -Value 0 -Type DWord")
        );
        assert!(script.contains(
            "Set-ItemProperty -Path $p -Name 'DnsOverHttpsMode' -Value 'off' -Type String"
        ));
        assert!(script.contains("Set-ItemProperty -Path $p -Name 'ClearBrowsingDataOnExitList' -Value @('a','b') -Type MultiString"));
        assert!(script.contains("Remove-ItemProperty -Path $p -Name 'AutofillAddressEnabled'"));
    }
}
