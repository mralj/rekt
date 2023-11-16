use serde::Deserialize;

const IP_API_URL: &str = "https://ipapi.co/";
const IP_API_AUTH_KEY: &str = "yAXXlrlnIkh0RO3G6rKivG2G4S1RFQIjymZitzIlmgif7mSprk";

pub(super) async fn get_ip_location_info(ip: &str) -> anyhow::Result<IpInfo> {
    let url = format!("{}{}/json?key={}", IP_API_URL, ip, IP_API_AUTH_KEY);
    let response = reqwest::get(&url).await?;
    if response.status().is_success() {
        Ok(response.json::<IpInfo>().await?)
    } else {
        Err(anyhow::anyhow!(
            "Failed to get ip info from ipapi.co: {}",
            response.status()
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct IpInfo {
    pub ip: String,
    pub city: String,
    pub country_name: String,
    pub org: String,
    pub country_code_iso3: String,
}

impl Default for IpInfo {
    fn default() -> Self {
        Self {
            ip: "N/A".into(),
            city: "N/A".into(),
            country_name: "N/A".into(),
            org: "N/A".into(),
            country_code_iso3: "N/A".into(),
        }
    }
}
