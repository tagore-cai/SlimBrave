use std::collections::BTreeMap;
use std::io::Read;

use crate::domain::catalog::{init as catalog, RemoteCatalog, RemotePolicy};
use crate::domain::Browser;

const CHROMIUM_SOURCE_URL: &str = "https://chromium.googlesource.com/chromium/src/+archive/refs/heads/main/components/policy/resources/templates/policy_definitions.tar.gz";
const BRAVE_SOURCE_URL: &str = "https://brave-browser-downloads.s3.brave.com/latest/policy_templates.zip";

fn remote_path(browser: Browser) -> Option<std::path::PathBuf> {
    dirs::cache_dir()
        .map(|cache| cache.join("slimbrave").join(browser.cache_file()))
}

fn decode_utf16_le(bytes: &[u8]) -> String {
    let bytes = if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    };
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

fn parse_adml(text: &str) -> BTreeMap<String, String> {
    let mut strings = BTreeMap::new();
    for m in text.split("<string id=").skip(1) {
        let Some((id, rest)) = m.split_once('>') else {
            continue;
        };
        let Some((value, _)) = rest.split_once("</string>") else {
            continue;
        };
        strings.insert(
            id.trim().trim_matches('"').to_owned(),
            value.trim().to_owned(),
        );
    }
    strings
}

fn parse_admx(text: &str) -> BTreeMap<String, String> {
    let mut policies = BTreeMap::new();
    for block in text.split("<policy ").skip(1) {
        let Some(end) = block.find("</policy>") else {
            continue;
        };
        let block = &block[..end];
        let Some(name) = extract_attr(block, "name") else {
            continue;
        };
        let elements = block
            .find("<elements>")
            .map(|start| &block[start + 10..])
            .and_then(|rest| rest.find("</elements>").map(|end| &rest[..end]))
            .unwrap_or("");
        let has_enabled = block.contains("<enabledValue>") || block.contains("<enabledList>");
        let kind = if elements.contains("<boolean ") {
            "Bool"
        } else if elements.contains("<decimal ") {
            "Int"
        } else if elements.contains("<list ") {
            "Array"
        } else if elements.contains("<text ") || elements.contains("<enum ") {
            "Str"
        } else if has_enabled {
            "Bool"
        } else {
            "unknown"
        };
        policies.insert(name, kind.to_owned());
    }
    policies
}

fn extract_attr(block: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = block.find(&needle)? + needle.len();
    let end = block[start..].find('"')? + start;
    Some(block[start..end].to_owned())
}

fn fetch_brave(
    keys: &[String],
    existing: &BTreeMap<String, (String, String)>,
) -> Result<BTreeMap<String, RemotePolicy>, String> {
    let data = download(BRAVE_SOURCE_URL)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data))
        .map_err(|err| format!("brave zip parse failed: {err}"))?;

    let admx_text = {
        let mut entry = archive
            .by_name("windows/admx/brave.admx")
            .map_err(|err| format!("brave.admx missing: {err}"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|err| err.to_string())?;
        decode_utf16_le(&bytes)
    };
    let adml_text = {
        let mut entry = archive
            .by_name("windows/admx/en-US/brave.adml")
            .map_err(|err| format!("brave.adml missing: {err}"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|err| err.to_string())?;
        decode_utf16_le(&bytes)
    };

    let types = parse_admx(&admx_text);
    let strings = parse_adml(&adml_text);

    let mut policies = BTreeMap::new();
    for key in keys {
        let Some(kind) = types.get(key) else {
            continue;
        };
        let name = strings
            .get(key)
            .cloned()
            .or_else(|| existing.get(key).map(|(n, _)| n.clone()))
            .unwrap_or_else(|| key.clone());
        let tooltip = strings
            .get(&format!("{key}_Explain"))
            .cloned()
            .or_else(|| existing.get(key).map(|(_, t)| t.clone()))
            .unwrap_or_default();
        policies.insert(
            key.clone(),
            RemotePolicy {
                name,
                tooltip,
                r#type: kind.clone(),
            },
        );
    }
    Ok(policies)
}

fn parse_simple_yaml(text: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent > 0 {
            continue;
        }
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() && i < lines.len() && lines[i].trim_start().starts_with("- ") {
            let mut items = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with("- ") {
                items.push(
                    lines[i]
                        .trim_start()[2..]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_owned(),
                );
                i += 1;
            }
            result.insert(key.trim().to_owned(), items.join(","));
        } else if matches!(rest, "|" | "|-" | "|+" | ">" | ">-") {
            let mut block = Vec::new();
            while i < lines.len() && (lines[i].starts_with(' ') || lines[i].trim().is_empty()) {
                block.push(lines[i]);
                i += 1;
            }
            result.insert(
                key.trim().to_owned(),
                block.join("\n").trim_end().to_owned(),
            );
        } else if rest.starts_with('[') {
            let items: Vec<String> = rest
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_owned())
                .filter(|item| !item.is_empty())
                .collect();
            result.insert(key.trim().to_owned(), items.join(","));
        } else if rest.starts_with("- ") {
            let mut items = vec![rest.trim_start_matches("- ").trim_matches('"').to_owned()];
            while i < lines.len() && lines[i].trim_start().starts_with("- ") {
                items.push(lines[i].trim_start()[2..].trim().trim_matches('"').to_owned());
                i += 1;
            }
            result.insert(key.trim().to_owned(), items.join(","));
        } else {
            let value = rest
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_owned();
            result.insert(key.trim().to_owned(), value);
        }
    }
    result
}

fn clean_desc(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let end = rest[start..].find('>').map(|p| start + p + 1);
        let Some(end) = end else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let tag = &rest[start..end];
        let is_ph = tag.starts_with("<ph") || tag.starts_with("</ph");
        let is_ex = tag.starts_with("<ex") || tag.starts_with("</ex");
        if !is_ph && !is_ex {
            out.push_str(tag);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out = out.replace("Google Chrome", "Brave").replace("Chrome", "Brave");
    out = out.replace('$', "").replace("1Brave", "Brave").replace("2Brave", "Brave");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_owned()
}

pub fn fetch_policies(browser: Browser) -> Result<usize, String> {
    let data = download(CHROMIUM_SOURCE_URL)?;
    let catalog = catalog();
    let keys: Vec<String> = catalog.features.iter().map(|f| f.key.clone()).collect();
    let existing: BTreeMap<String, (String, String)> = catalog
        .features
        .iter()
        .map(|f| (f.key.clone(), (f.name.clone(), f.tooltip.clone())))
        .collect();
    drop(catalog);

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
        chromium.insert(key, parse_simple_yaml(&content));
    }

    let mut policies = BTreeMap::new();
    for key in &keys {
        let Some(entry) = chromium.get(key) else {
            continue;
        };
        let name = entry
            .get("caption")
            .map(|c| clean_desc(c))
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| existing.get(key).map(|(n, _)| n.clone()).unwrap_or_default());
        let tooltip = entry
            .get("desc")
            .map(|d| clean_desc(d))
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| existing.get(key).map(|(_, t)| t.clone()).unwrap_or_default());
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
        match fetch_brave(&keys, &existing) {
            Ok(brave) => {
                brave_count = brave.len();
                policies.extend(brave);
            }
            Err(err) => {
                eprintln!("slimbrave: brave source skipped: {err}");
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

fn download(url: &str) -> Result<Vec<u8>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_scalars_and_blocks() {
        let yaml = r#"caption: Enable reporting of usage data
desc: |-
  When this policy is Enabled, data is reported.

  Second paragraph.
type: boolean
supported_on:
- chrome.win:50-
features:
  dynamic_refresh: true
"#;
        let parsed = parse_simple_yaml(yaml);
        assert_eq!(parsed.get("caption").map(String::as_str), Some("Enable reporting of usage data"));
        assert!(parsed.get("desc").unwrap().contains("Second paragraph"));
        assert_eq!(parsed.get("type").map(String::as_str), Some("boolean"));
        assert!(parsed.get("supported_on").unwrap().contains("chrome.win:50-"));
    }

    #[test]
    fn clean_desc_strips_markup_and_renames_product() {
        let desc = "About <ph name=\"PRODUCT_NAME\">$1<ex>Google Chrome</ex></ph> data\nfor $2 reports.";
        let cleaned = clean_desc(desc);
        assert!(!cleaned.contains('<'));
        assert!(cleaned.contains("Brave"));
        assert!(!cleaned.contains("$"));
    }

    #[test]
    fn clean_desc_handles_empty() {
        assert_eq!(clean_desc(""), "");
    }
}

#[test]
#[ignore = "requires network"]
fn fetch_policies_network_end_to_end() {
    let brave = fetch_policies(Browser::Brave).expect("brave fetch should succeed");
    assert!(brave >= 43, "brave expected at least 43 matched policies, got {brave}");
    let path = remote_path(Browser::Brave).unwrap();
    let json = std::fs::read_to_string(&path).expect("brave remote catalog written");
    let remote: crate::domain::catalog::RemoteCatalog =
        serde_json::from_str(&json).expect("remote catalog parses");
    assert!(remote.policies.contains_key("BraveRewardsDisabled"), "brave-specific policy missing");
    assert!(remote.policies.contains_key("MetricsReportingEnabled"));

    let chrome = fetch_policies(Browser::Chrome).expect("chrome fetch should succeed");
    assert!(chrome >= 28, "chrome expected at least 28 matched policies, got {chrome}");
    let path = remote_path(Browser::Chrome).unwrap();
    let json = std::fs::read_to_string(&path).expect("chrome remote catalog written");
    let remote: crate::domain::catalog::RemoteCatalog =
        serde_json::from_str(&json).expect("remote catalog parses");
    assert!(!remote.policies.contains_key("BraveRewardsDisabled"), "brave policy must not leak into chrome");
}
