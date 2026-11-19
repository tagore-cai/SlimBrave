//! 从 Brave / Chromium 官方源拉取最新策略模板元数据。
//!
//! 仅对非 Windows 目标编译（Windows 端使用 `tools/fetch_policies.py`）。
//! - [`admx`]：Brave 的 ADMX / ADML 解析
//! - [`yaml`]：Chromium 的 YAML 定义解析

mod admx;
mod yaml;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::Read;

use crate::domain::catalog::{init as catalog, RemoteCatalog, RemotePolicy};
use crate::domain::Browser;

pub(super) const CHROMIUM_SOURCE_URL: &str = "https://chromium.googlesource.com/chromium/src/+archive/refs/heads/main/components/policy/resources/templates/policy_definitions.tar.gz";
pub(super) const BRAVE_SOURCE_URL: &str =
    "https://brave-browser-downloads.s3.brave.com/latest/policy_templates.zip";

fn remote_path(browser: Browser) -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|cache| cache.join("slimbrave").join(browser.cache_file()))
}

/// 构建本次拉取的目标键集与兜底元数据，只含当前浏览器适用的策略。
fn fetch_scope(browser: Browser) -> (Vec<String>, BTreeMap<String, (String, String)>) {
    let catalog = catalog();
    let keys: Vec<String> = catalog
        .features_for(browser)
        .map(|(_, f)| f.key.clone())
        .collect();
    let existing: BTreeMap<String, (String, String)> = catalog
        .features_for(browser)
        .map(|(_, f)| (f.key.clone(), (f.name.clone(), f.tooltip.clone())))
        .collect();
    drop(catalog);
    (keys, existing)
}

pub fn fetch_policies(browser: Browser) -> Result<usize, String> {
    let data = download(CHROMIUM_SOURCE_URL)?;
    let (keys, existing) = fetch_scope(browser);

    // Chromium definitions (fallback source)
    let mut chromium = BTreeMap::new();
    let mut decoder = flate2::read::GzDecoder::new(data.as_slice());
    let mut tar = tar::Archive::new(&mut decoder);
    for entry in tar.entries().map_err(|err| err.to_string())? {
        let Ok(mut entry) = entry else {
            continue;
        };
        let path = entry.path().map_err(|err| err.to_string())?;
        let Some(name) = path.file_name() else {
            continue;
        };
        let Some(name) = name.to_str() else {
            continue;
        };
        let key = name.strip_suffix(".yaml").unwrap_or(name).to_owned();
        if !keys.iter().any(|k| k == &key) {
            continue;
        }
        let mut content = String::new();
        if entry.read_to_string(&mut content).is_err() {
            continue;
        }
        chromium.insert(key, yaml::parse_simple_yaml(&content));
    }

    let mut policies = BTreeMap::new();
    for key in &keys {
        let Some(entry) = chromium.get(key) else {
            continue;
        };
        let name = entry
            .get("caption")
            .map(|c| yaml::clean_desc(c))
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| {
                existing
                    .get(key)
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default()
            });
        let tooltip = entry
            .get("desc")
            .map(|d| yaml::clean_desc(d))
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| {
                existing
                    .get(key)
                    .map(|(_, t)| t.clone())
                    .unwrap_or_default()
            });
        policies.insert(
            key.clone(),
            RemotePolicy {
                name,
                tooltip,
                r#type: entry
                    .get("type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_owned()),
            },
        );
    }

    // Brave official definitions override Chromium where available.
    let mut brave_count = 0;
    if browser == Browser::Brave {
        match admx::fetch_brave(&keys, &existing) {
            Ok(brave) => {
                brave_count = brave.len();
                policies.extend(brave);
            }
            Err(err) => {
                crate::logging::log(&format!("brave source skipped: {err}"));
            }
        }
    }

    let remote = RemoteCatalog {
        version: match browser {
            Browser::Brave => "brave-official + chromium-main".to_owned(),
            Browser::Chrome => "chromium-main".to_owned(),
        },
        source: match browser {
            Browser::Brave => format!("{CHROMIUM_SOURCE_URL}\n{BRAVE_SOURCE_URL}"),
            Browser::Chrome => CHROMIUM_SOURCE_URL.to_owned(),
        },
        policies,
    };
    let json = serde_json::to_string_pretty(&remote).map_err(|err| err.to_string())?;

    let path = remote_path(browser).ok_or("could not resolve cache directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    std::fs::write(&path, json).map_err(|err| err.to_string())?;
    let _ = brave_count;
    Ok(remote.policies.len())
}

pub(super) fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .user_agent("slimbrave-fetch/1.0")
        .timeout(std::time::Duration::from_secs(60))
        .build();
    let resp = agent
        .get(url)
        .call()
        .map_err(|err| format!("download failed: {err}"))?;
    let mut data = Vec::new();
    const MAX: u64 = 30 * 1024 * 1024;
    resp.into_reader()
        .take(MAX + 1)
        .read_to_end(&mut data)
        .map_err(|err| err.to_string())?;
    if data.len() as u64 > MAX {
        return Err(format!("response exceeds {MAX} bytes"));
    }
    Ok(data)
}
