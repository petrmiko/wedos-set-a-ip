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

#[derive(Debug)]
enum WapiCommand {
    ListDomains,
    DnsRowsList(WapiDsnRowsListData),
    DnsRowUpdate(WapiDnsRowUpdateData),
}

impl WapiCommand {
    fn api_key(&self) -> &'static str {
        match self {
            WapiCommand::ListDomains => "dns-domains-list",
            WapiCommand::DnsRowsList(_) => "dns-rows-list",
            WapiCommand::DnsRowUpdate(_) => "dns-row-update",
        }
    }

    fn get_request(&self) -> WapiRequest {
        let credentials = get_credentials();

        let request = WapiRequest {
            user: credentials.0,
            auth: credentials.1,

            test: env::var("TEST").unwrap_or("0".to_string()),
            command: self.api_key(),
            data: match self {
                WapiCommand::ListDomains => json!({}),
                WapiCommand::DnsRowsList(data) => json!(data),
                WapiCommand::DnsRowUpdate(data) => json!(data),
            },
        };

        return request;
    }
}

#[derive(Debug, Serialize)]
struct WapiRequest {
    user: String,
    auth: String,
    test: String,
    command: &'static str,
    data: Value,
}

#[derive(Debug, Serialize)]
struct WapiPayload {
    request: WapiRequest,
}

fn get_credentials() -> (String, String) {
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

    return (wapi_user, wapi_auth);
}

pub fn update_a_record(ipv4: String) {
    let reqwest_client = reqwest::blocking::Client::new();
    let wapi_url =
        env::var("WEDOS_API_URL").unwrap_or(String::from("https://api.wedos.com/wapi/json"));

    // let command = WapiCommand::ListDomains;

    // let command = WapiCommand::DnsRowsList(WapiDsnRowsListData {
    //     domain: env::var("DOMAIN").unwrap(),
    // });

    let command = WapiCommand::DnsRowUpdate(WapiDnsRowUpdateData {
        domain: env::var("DOMAIN").unwrap(),
        row_id: env::var("DNS_ROW_ID").unwrap(),
        ttl: "300".to_string(),
        rdata: ipv4,
    });

    let request = command.get_request();
    let payload = WapiPayload { request };
    let mut payload_map = HashMap::new();
    payload_map.insert("request", json!(payload).to_string());

    println!("{:?}", payload);
    let response = reqwest_client
        .post(wapi_url)
        .form(&payload_map)
        .send()
        .unwrap()
        .json::<Value>();
    println!("{:#?}", response);
}
