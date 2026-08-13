use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::domain::WriteValue;
use crate::infrastructure::platform::Channel;

fn profile_identifier(domain: &str) -> String {
    format!("com.slimbrave.profile.{domain}")
}

fn stable_uuid(identifier: &str) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, identifier.as_bytes())
        .to_string()
        .to_uppercase()
}

pub fn build_mobileconfig(
    domain: &str,
    payload: &BTreeMap<String, WriteValue>,
) -> plist::Value {
    let identifier = profile_identifier(domain);
    let profile_uuid = stable_uuid(&identifier);
    let payload_uuid = stable_uuid(&format!("{identifier}.payload"));

    let mut mcx_settings = plist::Dictionary::new();
    for (key, value) in payload {
        mcx_settings.insert(key.clone(), to_plist_value(value));
    }

    let mut forced_payload = plist::Dictionary::new();
    forced_payload.insert("mcx_preference_settings".to_owned(), plist::Value::Dictionary(mcx_settings));

    let mut domain_content = plist::Dictionary::new();
    domain_content.insert(
        domain.to_owned(),
        plist::Value::Dictionary({
            let mut d = plist::Dictionary::new();
            d.insert("Forced".to_owned(), plist::Value::Array(vec![plist::Value::Dictionary(forced_payload)]));
            d
        }),
    );

    let mut main_payload = plist::Dictionary::new();
    main_payload.insert("PayloadType".to_owned(), plist::Value::String("com.apple.ManagedClient.preferences".to_owned()));
    main_payload.insert("PayloadVersion".to_owned(), plist::Value::Integer(1.into()));
    main_payload.insert("PayloadIdentifier".to_owned(), plist::Value::String(format!("{identifier}.payload")));
    main_payload.insert("PayloadUUID".to_owned(), plist::Value::String(payload_uuid));
    main_payload.insert("PayloadEnabled".to_owned(), plist::Value::Boolean(true));
    main_payload.insert("PayloadContent".to_owned(), plist::Value::Dictionary(domain_content));

    let mut root = plist::Dictionary::new();
    root.insert("PayloadContent".to_owned(), plist::Value::Array(vec![plist::Value::Dictionary(main_payload)]));
    root.insert("PayloadDisplayName".to_owned(), plist::Value::String(format!("SlimBrave Policy ({domain})")));
    root.insert("PayloadIdentifier".to_owned(), plist::Value::String(identifier));
    root.insert("PayloadType".to_owned(), plist::Value::String("Configuration".to_owned()));
    root.insert("PayloadUUID".to_owned(), plist::Value::String(profile_uuid));
    root.insert("PayloadVersion".to_owned(), plist::Value::Integer(1.into()));
    root.insert("PayloadScope".to_owned(), plist::Value::String("System".to_owned()));

    plist::Value::Dictionary(root)
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

fn to_plist_value(value: &WriteValue) -> plist::Value {
    match value {
        WriteValue::Bool(value) => plist::Value::Boolean(*value),
        WriteValue::Int(value) => plist::Value::Integer((*value).into()),
        WriteValue::Str(value) => plist::Value::String(value.clone()),
        WriteValue::Array(items) => plist::Value::Array(
            items
                .iter()
                .map(|item| plist::Value::String(item.clone()))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> BTreeMap<String, WriteValue> {
        let mut map = BTreeMap::new();
        map.insert(
            "BraveRewardsDisabled".to_owned(),
            WriteValue::Bool(true),
        );
        map.insert(
            "BrowserSignin".to_owned(),
            WriteValue::Int(0),
        );
        map.insert(
            "DnsOverHttpsMode".to_owned(),
            WriteValue::Str("off".to_owned()),
        );
        map.insert(
            "ClearBrowsingDataOnExitList".to_owned(),
            WriteValue::Array(vec!["browsing_history".to_owned(), "cookies_and_other_site_data".to_owned()]),
        );
        map
    }

    #[test]
    fn mobileconfig_structure_is_valid() {
        let domain = "com.brave.Browser";
        let config = build_mobileconfig(domain, &payload());
        let dict = config.as_dictionary().unwrap();

        assert_eq!(dict.get("PayloadType").unwrap().as_string().unwrap(), "Configuration");
        assert_eq!(dict.get("PayloadScope").unwrap().as_string().unwrap(), "System");
        assert_eq!(
            dict.get("PayloadIdentifier").unwrap().as_string().unwrap(),
            "com.slimbrave.profile.com.brave.Browser"
        );
        assert_eq!(
            dict.get("PayloadDisplayName").unwrap().as_string().unwrap(),
            "SlimBrave Policy (com.brave.Browser)"
        );

        let content = dict.get("PayloadContent").unwrap().as_array().unwrap();
        let main = content[0].as_dictionary().unwrap();
        assert_eq!(
            main.get("PayloadType").unwrap().as_string().unwrap(),
            "com.apple.ManagedClient.preferences"
        );
        let domain_dict = main.get("PayloadContent").unwrap().as_dictionary().unwrap();
        let forced = domain_dict
            .get(domain)
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Forced")
            .unwrap()
            .as_array()
            .unwrap();
        let mcx = forced[0]
            .as_dictionary()
            .unwrap()
            .get("mcx_preference_settings")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(
            mcx.get("BraveRewardsDisabled").unwrap().as_boolean().unwrap(),
            true
        );
        assert_eq!(mcx.get("BrowserSignin").unwrap().as_signed_integer().unwrap(), 0);
        assert_eq!(mcx.get("DnsOverHttpsMode").unwrap().as_string().unwrap(), "off");
        assert_eq!(
            mcx.get("ClearBrowsingDataOnExitList")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn uuids_are_stable() {
        let domain = "com.brave.Browser";
        let a = build_mobileconfig(domain, &payload());
        let b = build_mobileconfig(domain, &payload());
        assert_eq!(
            a.as_dictionary().unwrap().get("PayloadUUID"),
            b.as_dictionary().unwrap().get("PayloadUUID")
        );
    }

    #[test]
    fn writes_mobileconfig_file() {
        let temp = tempfile::tempdir().unwrap();
        let channel = crate::infrastructure::platform::CHANNELS[0];
        let path = write_mobileconfig(&channel, &payload(), Some(temp.path())).unwrap();
        assert!(path.exists());
        let parsed = plist::Value::from_file(&path).unwrap();
        assert!(parsed.as_dictionary().unwrap().contains_key("PayloadContent"));
    }
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
