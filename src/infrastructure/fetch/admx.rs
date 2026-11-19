//! Brave 官方 ADMX / ADML 模板解析。

use std::collections::BTreeMap;
use std::io::Read;

use crate::domain::catalog::RemotePolicy;

use super::{download, BRAVE_SOURCE_URL};

pub(super) fn decode_utf16_le(bytes: &[u8]) -> String {
    let bytes = if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    };
    // 用 chunks(2) + try_into 替代 chunks_exact：行为一致（忽略尾部不成对字节），
    // 且不依赖 1.88 才稳定的 as_chunks，保持项目声明的 MSRV 1.75。
    let units: Vec<u16> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            let pair: [u8; 2] = chunk.try_into().ok()?;
            Some(u16::from_le_bytes(pair))
        })
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

pub(super) fn fetch_brave(
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
        entry
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        decode_utf16_le(&bytes)
    };
    let adml_text = {
        let mut entry = archive
            .by_name("windows/admx/en-US/brave.adml")
            .map_err(|err| format!("brave.adml missing: {err}"))?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
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
