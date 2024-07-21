use serde::Serialize;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, env};

#[derive(Debug, Serialize)]
struct WapiDsnRowsListData {
    domain: String,
}

#[derive(Debug, Serialize)]
struct WapiDnsRowUpdateData {
    domain: String,
    row_id: String,
    ttl: String,
    rdata: String,
}

enum WapiCommand {
    ListDomains,
    DnsRowsList,
    DnsRowUpdate,
}

impl WapiCommand {
    fn key(&self) -> String {
        match self {
            WapiCommand::ListDomains => "dns-domains-list".to_string(),
            WapiCommand::DnsRowsList => "dns-rows-list".to_string(),
            WapiCommand::DnsRowUpdate => "dns-row-update".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct WapiRequest {
    user: String,
    auth: String,
    test: String,
    command: String,
    data: WapiDnsRowUpdateData, // TODO learn about how to handle this in a generic way based on command
}

#[derive(Debug, Serialize)]
struct WapiPayload {
    request: WapiRequest,
}

fn get_request(command: WapiCommand, data: WapiDnsRowUpdateData) -> WapiRequest {
    let current_hour_prague = chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Prague)
        .format("%H");

    let wapi_user = env::var("WEDOS_USER").unwrap();
    let wapi_password = env::var("WEDOS_PASSWORD").unwrap();

    let mut password_hasher = Sha1::new();
    let mut auth_hasher = Sha1::new();
    password_hasher.update(wapi_password);
    let pass_hash = password_hasher.finalize();

    let wapi_auth_raw = format!(
        "{}{}{}",
        wapi_user,
        format!("{:x}", pass_hash),
        current_hour_prague
    );
    auth_hasher.update(wapi_auth_raw);
    let wapi_auth = format!("{:x}", auth_hasher.finalize());

    return WapiRequest {
        user: wapi_user,
        auth: wapi_auth,
        test: env::var("TEST").unwrap_or("0".to_string()),
        command: command.key(),
        data,
    };
}

pub fn update_a_record(ipv4: String) {
    let reqwest_client = reqwest::blocking::Client::new();
    let wapi_url =
        env::var("WEDOS_API_URL").unwrap_or(String::from("https://api.wedos.com/wapi/json"));

    let command = WapiCommand::DnsRowUpdate;
    let data = WapiDnsRowUpdateData {
        domain: env::var("DOMAIN").unwrap(),
        row_id: env::var("DNS_ROW_ID").unwrap(),
        ttl: "300".to_string(),
        rdata: ipv4,
    };

    let request = get_request(command, data);
    let payload = WapiPayload { request };
    let mut payload_map = HashMap::new();
    payload_map.insert("request", json!(payload).to_string());

    let response = reqwest_client
        .post(wapi_url)
        .form(&payload_map)
        .send()
        .unwrap()
        .json::<Value>();
    println!("{:#?}", response);
}
