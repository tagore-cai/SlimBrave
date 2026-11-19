use super::{build_mobileconfig, write_mobileconfig};
use crate::domain::WriteValue;
use std::collections::BTreeMap;

fn payload() -> BTreeMap<String, WriteValue> {
    let mut map = BTreeMap::new();
    map.insert("BraveRewardsDisabled".to_owned(), WriteValue::Bool(true));
    map.insert("BrowserSignin".to_owned(), WriteValue::Int(0));
    map.insert(
        "DnsOverHttpsMode".to_owned(),
        WriteValue::Str("off".to_owned()),
    );
    map.insert(
        "ClearBrowsingDataOnExitList".to_owned(),
        WriteValue::Array(vec![
            "browsing_history".to_owned(),
            "cookies_and_other_site_data".to_owned(),
        ]),
    );
    map
}

#[test]
fn mobileconfig_structure_is_valid() {
    let domain = "com.brave.Browser";
    let config = build_mobileconfig(domain, &payload());
    let dict = config.as_dictionary().unwrap();

    assert_eq!(
        dict.get("PayloadType").unwrap().as_string().unwrap(),
        "Configuration"
    );
    assert_eq!(
        dict.get("PayloadScope").unwrap().as_string().unwrap(),
        "System"
    );
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
        mcx.get("BraveRewardsDisabled")
            .unwrap()
            .as_boolean()
            .unwrap(),
        true
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
    assert!(parsed
        .as_dictionary()
        .unwrap()
        .contains_key("PayloadContent"));
}
