use std::env;

const IP_URL: &str = "https://ipinfo.io/ip";

pub fn get_public_ipv4() -> String {
    let ip_provider_url = env::var("IP_PROVIDER_URL").unwrap_or(IP_URL.to_string());
    reqwest::blocking::get(ip_provider_url)
        .unwrap()
        .text()
        .unwrap()
}
