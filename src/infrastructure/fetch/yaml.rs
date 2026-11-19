//! Chromium 策略定义的轻量 YAML 解析与描述清洗。

use std::collections::BTreeMap;

pub(super) fn parse_simple_yaml(text: &str) -> BTreeMap<String, String> {
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
                    lines[i].trim_start()[2..]
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
                items.push(
                    lines[i].trim_start()[2..]
                        .trim()
                        .trim_matches('"')
                        .to_owned(),
                );
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

pub(super) fn clean_desc(text: &str) -> String {
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
    out = out
        .replace("Google Chrome", "Brave")
        .replace("Chrome", "Brave");
    out = out
        .replace('$', "")
        .replace("1Brave", "Brave")
        .replace("2Brave", "Brave");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_owned()
}
