//! 纯函数：把载荷构造成合法的 .mobileconfig plist 树。

use std::collections::BTreeMap;

use uuid::Uuid;

use crate::domain::WriteValue;

fn profile_identifier(domain: &str) -> String {
    format!("com.slimbrave.profile.{domain}")
}

fn stable_uuid(identifier: &str) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, identifier.as_bytes())
        .to_string()
        .to_uppercase()
}

pub fn build_mobileconfig(domain: &str, payload: &BTreeMap<String, WriteValue>) -> plist::Value {
    let identifier = profile_identifier(domain);
    let profile_uuid = stable_uuid(&identifier);
    let payload_uuid = stable_uuid(&format!("{identifier}.payload"));

    let mut mcx_settings = plist::Dictionary::new();
    for (key, value) in payload {
        mcx_settings.insert(key.clone(), to_plist_value(value));
    }

    let mut forced_payload = plist::Dictionary::new();
    forced_payload.insert(
        "mcx_preference_settings".to_owned(),
        plist::Value::Dictionary(mcx_settings),
    );

    let mut domain_content = plist::Dictionary::new();
    domain_content.insert(
        domain.to_owned(),
        plist::Value::Dictionary({
            let mut d = plist::Dictionary::new();
            d.insert(
                "Forced".to_owned(),
                plist::Value::Array(vec![plist::Value::Dictionary(forced_payload)]),
            );
            d
        }),
    );

    let mut main_payload = plist::Dictionary::new();
    main_payload.insert(
        "PayloadType".to_owned(),
        plist::Value::String("com.apple.ManagedClient.preferences".to_owned()),
    );
    main_payload.insert("PayloadVersion".to_owned(), plist::Value::Integer(1.into()));
    main_payload.insert(
        "PayloadIdentifier".to_owned(),
        plist::Value::String(format!("{identifier}.payload")),
    );
    main_payload.insert("PayloadUUID".to_owned(), plist::Value::String(payload_uuid));
    main_payload.insert("PayloadEnabled".to_owned(), plist::Value::Boolean(true));
    main_payload.insert(
        "PayloadContent".to_owned(),
        plist::Value::Dictionary(domain_content),
    );

    let mut root = plist::Dictionary::new();
    root.insert(
        "PayloadContent".to_owned(),
        plist::Value::Array(vec![plist::Value::Dictionary(main_payload)]),
    );
    root.insert(
        "PayloadDisplayName".to_owned(),
        plist::Value::String(format!("SlimBrave Policy ({domain})")),
    );
    root.insert(
        "PayloadIdentifier".to_owned(),
        plist::Value::String(identifier),
    );
    root.insert(
        "PayloadType".to_owned(),
        plist::Value::String("Configuration".to_owned()),
    );
    root.insert("PayloadUUID".to_owned(), plist::Value::String(profile_uuid));
    root.insert("PayloadVersion".to_owned(), plist::Value::Integer(1.into()));
    root.insert(
        "PayloadScope".to_owned(),
        plist::Value::String("System".to_owned()),
    );

    plist::Value::Dictionary(root)
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
