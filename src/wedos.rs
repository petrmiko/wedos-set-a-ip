use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::env;
use std::net::Ipv4Addr;

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

    fn get_request(&self) -> Result<WapiRequest> {
        let (user, auth) = get_credentials()?;

        Ok(WapiRequest {
            user,
            auth,
            test: env::var("TEST").unwrap_or("0".to_string()),
            command: self.api_key(),
            data: match self {
                WapiCommand::ListDomains => json!({}),
                WapiCommand::DnsRowsList(data) => json!(data),
                WapiCommand::DnsRowUpdate(data) => json!(data),
            },
        })
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

fn get_credentials() -> Result<(String, String)> {
    let current_hour_prague = chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Prague)
        .format("%H");

    let wapi_user = env::var("WEDOS_USER").context("WEDOS_USER not set")?;
    let wapi_password = env::var("WEDOS_PASSWORD").context("WEDOS_PASSWORD not set")?;

    let mut password_hasher = Sha1::new();
    let mut auth_hasher = Sha1::new();
    password_hasher.update(wapi_password);
    let pass_hash = password_hasher.finalize();

    let wapi_auth_raw = format!("{wapi_user}{pass_hash:x}{current_hour_prague}");
    auth_hasher.update(wapi_auth_raw);
    let wapi_auth = format!("{:x}", auth_hasher.finalize());

    Ok((wapi_user, wapi_auth))
}

const WAPI_OK: u64 = 1000;
const DEFAULT_TTL: u16 = 300;

fn check_code(response: &Value) -> Result<()> {
    let inner = response
        .get("response")
        .context("WAPI response missing `response` key")?;
    let code = inner
        .get("code")
        .and_then(Value::as_u64)
        .context("WAPI response missing `code`")?;

    if code == WAPI_OK {
        return Ok(());
    }

    let result = inner
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("no result message");
    anyhow::bail!("WAPI error {code}: {result}");
}

fn rows(response: &Value) -> impl Iterator<Item = &Value> {
    response
        .pointer("/response/data/row")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect::<Vec<_>>())
        .or_else(|| {
            let data = response.pointer("/response/data")?.as_object()?;
            Some(
                data.values()
                    .filter(|row| row.get("ID").is_some())
                    .collect(),
            )
        })
        .unwrap_or_default()
        .into_iter()
}

fn field(row: &Value, key: &str) -> Option<String> {
    let value = row.get(key)?;
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_u64().map(|number| number.to_string()))
}

fn find_row<'a>(response: &'a Value, row_id: &str) -> Option<&'a Value> {
    rows(response).find(|row| field(row, "ID").as_deref() == Some(row_id))
}

fn make_request(command: WapiCommand) -> Result<Value> {
    let wapi_url =
        env::var("WEDOS_API_URL").unwrap_or(String::from("https://api.wedos.com/wapi/json"));

    let request = command.get_request()?;
    let payload = WapiPayload { request };

    let response: Value = ureq::post(&wapi_url)
        .send_form([("request", json!(payload).to_string().as_str())])
        .with_context(|| format!("calling WAPI at {wapi_url}"))?
        .body_mut()
        .read_json()
        .context("decoding WAPI response")?;

    check_code(&response)?;

    Ok(response)
}

pub fn list_domains() -> Result<Value> {
    make_request(WapiCommand::ListDomains)
}

pub fn list_dns_rows(domain: String) -> Result<Value> {
    make_request(WapiCommand::DnsRowsList(WapiDsnRowsListData { domain }))
}

pub fn update_a_record_if_changed(ipv4: Ipv4Addr, domain: String, row_id: String) -> Result<bool> {
    let listing = list_dns_rows(domain.clone())?;
    let rdata = ipv4.to_string();
    let ttl = DEFAULT_TTL.to_string();

    match find_row(&listing, &row_id) {
        Some(row) => {
            let rdtype = field(row, "rdtype").unwrap_or_default();
            anyhow::ensure!(
                rdtype == "A",
                "DNS row {row_id} of {domain} is an {rdtype} record, refusing to write an IPv4 to it"
            );

            if field(row, "rdata").as_deref() == Some(rdata.as_str())
                && field(row, "ttl").as_deref() == Some(ttl.as_str())
            {
                return Ok(false);
            }
        }
        None => eprintln!(
            "warning: DNS row {row_id} not listed for {domain}, updating without comparing"
        ),
    }

    make_request(WapiCommand::DnsRowUpdate(WapiDnsRowUpdateData {
        domain,
        row_id,
        ttl,
        rdata,
    }))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: Value, rdata: &str) -> Value {
        json!({ "ID": id, "name": "", "ttl": "300", "rdtype": "A", "rdata": rdata })
    }

    fn array_shape() -> Value {
        json!({ "response": { "code": 1000, "data": {
            "row": [row(json!("1"), "1.1.1.1"), row(json!("42"), "9.9.9.9")] } } })
    }

    fn object_shape() -> Value {
        json!({ "response": { "code": 1000, "data": {
            "row1": row(json!("1"), "1.1.1.1"), "row2": row(json!("42"), "9.9.9.9") } } })
    }

    fn rdata_of(response: &Value, row_id: &str) -> Option<String> {
        field(find_row(response, row_id)?, "rdata")
    }

    #[test]
    fn finds_row_in_array_shape() {
        assert_eq!(rdata_of(&array_shape(), "42").as_deref(), Some("9.9.9.9"));
    }

    #[test]
    fn finds_row_in_object_shape() {
        assert_eq!(rdata_of(&object_shape(), "42").as_deref(), Some("9.9.9.9"));
    }

    #[test]
    fn finds_row_when_wapi_sends_id_as_a_number() {
        let numeric = json!({ "response": { "code": 1000, "data": {
            "row": [row(json!(42), "9.9.9.9")] } } });

        assert_eq!(rdata_of(&numeric, "42").as_deref(), Some("9.9.9.9"));
    }

    #[test]
    fn reads_ttl_whether_string_or_number() {
        assert_eq!(
            field(&json!({ "ttl": "300" }), "ttl").as_deref(),
            Some("300")
        );
        assert_eq!(field(&json!({ "ttl": 300 }), "ttl").as_deref(), Some("300"));
    }

    #[test]
    fn missing_row_id_yields_none() {
        assert!(find_row(&array_shape(), "999").is_none());
        assert!(find_row(&json!({ "response": { "code": 1000 } }), "42").is_none());
    }

    #[test]
    fn ok_code_passes() {
        assert!(check_code(&json!({ "response": { "code": 1000, "result": "OK" } })).is_ok());
    }

    #[test]
    fn error_code_reports_code_and_result() {
        let err = check_code(&json!({
            "response": { "code": 2051, "result": "Access not allowed from this IP address" }
        }))
        .unwrap_err()
        .to_string();

        assert!(err.contains("2051"), "{err}");
        assert!(err.contains("Access not allowed"), "{err}");
    }

    #[test]
    fn malformed_response_is_an_error() {
        assert!(check_code(&json!({ "nonsense": true })).is_err());
    }
}
