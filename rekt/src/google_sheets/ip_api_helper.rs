use serde::Deserialize;

const IP_API_URL: &str = "https://ipapi.co/";
const IP_API_AUTH_KEY: &str = "yAXXlrlnIkh0RO3G6rKivG2G4S1RFQIjymZitzIlmgif7mSprk";

#[derive(Debug, Deserialize)]
pub struct IpInfo {
    pub ip: String,
    pub city: String,
    pub country_name: String,
    pub org: String,
    pub country_code_iso3: String,
}

pub async fn get_ip_info(ip: &str) -> anyhow::Result<()> {
    let url = format!("{}{}/json?key={}", IP_API_URL, ip, IP_API_AUTH_KEY);
    match reqwest::get(&url).await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("{:#?}", resp.json::<IpInfo>().await);
                //println!("{:#?}", resp.text().await?)
            } else {
                println!("Error: {:#?}", resp.status());
            }
            Ok(())
        }
        Err(e) => {
            println!("Error: {:#?}", e);
            Err(anyhow::anyhow!("Error: {:#?}", e))
        }
    }
}
