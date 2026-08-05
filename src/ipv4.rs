use anyhow::{Context, Result};
use std::env;
use std::net::Ipv4Addr;

const IP_URL: &str = "https://ipinfo.io/ip";

pub fn get_public_ipv4() -> Result<Ipv4Addr> {
    let ip_provider_url = env::var("IP_PROVIDER_URL").unwrap_or(IP_URL.to_string());
    let body = ureq::get(&ip_provider_url)
        .call()
        .with_context(|| format!("requesting public IP from {ip_provider_url}"))?
        .body_mut()
        .read_to_string()
        .context("reading public IP response body")?;

    let body = body.trim();
    body.parse().with_context(|| {
        let excerpt: String = body.chars().take(80).collect();
        format!("{ip_provider_url} returned {excerpt:?}, not an IPv4 address")
    })
}
