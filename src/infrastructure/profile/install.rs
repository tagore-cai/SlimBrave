//! 配置描述文件的落盘与系统交互（`open` / `profiles` 命令）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::domain::WriteValue;
use crate::infrastructure::platform::Channel;

use super::build_mobileconfig;

fn profile_identifier(domain: &str) -> String {
    format!("com.slimbrave.profile.{domain}")
}

pub fn write_mobileconfig(
    channel: &Channel,
    payload: &BTreeMap<String, WriteValue>,
    target_dir: Option<&Path>,
) -> Result<PathBuf> {
    let dir = match target_dir {
        Some(path) => path.to_path_buf(),
        None => dirs::desktop_dir().context("could not resolve desktop directory")?,
    };
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("SlimBrave-{}.mobileconfig", channel.domain));
    build_mobileconfig(channel.domain, payload).to_file_xml(&path)?;
    Ok(path)
}

pub fn open_config(path: &Path) -> Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .status()
        .context("could not open configuration profile")?;
    Ok(())
}

pub fn remove_profile(channel: &Channel) -> bool {
    let identifier = profile_identifier(channel.domain);
    std::process::Command::new("profiles")
        .args(["remove", "-identifier", &identifier, "-forced"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn profile_installed(channel: &Channel) -> bool {
    let identifier = profile_identifier(channel.domain);
    for args in [
        vec!["list"],
        vec!["list", "-output", "stdout-xml", "-type", "configuration"],
    ] {
        let Ok(output) = std::process::Command::new("profiles").args(&args).output() else {
            continue;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains(&identifier) || text.contains("SlimBrave Policy") {
            return true;
        }
    }
    false
}
