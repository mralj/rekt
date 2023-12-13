pub mod ip_api_helper;

use chrono::Utc;
use google_sheets4::{
    api::ValueRange,
    hyper::{client::HttpConnector, Client},
    hyper_rustls::{self, HttpsConnector},
    oauth2::{ServiceAccountAuthenticator, ServiceAccountKey},
    Sheets,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{cli::Cli, eth::transactions::decoder::BuyTokenInfo, p2p::Peer, utils::helpers};

use self::ip_api_helper::get_ip_location_info;

const SPREADSHEET_ID: &str = "1o656_BLxhxnU4ovssiZv41BLqhCRT5qMcSVp1hojPfM";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogToSheets {
    pub token_address: String,
    pub liquidity_hash: String,
    pub liq_received_at: String,
    pub liq_received_at_unix: u64,
    pub was_tx_direct: bool,

    pub server_index: usize,
    pub is_server_important: bool,
    pub name: String,
    pub our_country: String,
    pub our_city: String,
    pub our_td: usize,

    pub peer_td: u64,
    pub peer_enode: String,
    pub peer_info: String,
    pub peer_country: String,
    pub peer_city: String,
    pub peer_server: String,

    pub start_wallet: String,
    pub end_wallet: String,

    pub batch_num: u8,

    bsc_scan_token_url: String,
    bsc_scan_liquidity_url: String,
}

impl LogToSheets {
    pub async fn new(cli: &Cli, peer: &Peer, buy_info: &BuyTokenInfo) -> Self {
        let start_wallet = match cli.first_wallet {
            Some(wallet) => format!("{:#x}", wallet),
            None => "N/A".into(),
        };
        let end_wallet = match cli.last_wallet {
            Some(wallet) => format!("{:#x}", wallet),
            None => "N/A".into(),
        };

        let peer_location_info = get_ip_location_info(&peer.node_record.ip)
            .await
            .unwrap_or_default();

        Self {
            token_address: format!("{:#x}", buy_info.token.buy_token_address),
            liquidity_hash: format!("{:#x}", buy_info.hash),
            liq_received_at: buy_info.time.format("%Y-%m-%d %H:%M:%S:%f").to_string(),
            liq_received_at_unix: buy_info.time.timestamp_micros() as u64,
            was_tx_direct: buy_info.was_tx_direct,
            is_server_important: !cli.is_un_important_server,
            server_index: cli.server_index,
            name: cli.name.clone(),
            our_country: cli.country.clone(),
            our_city: cli.city.clone(),
            our_td: 0,
            peer_td: peer.td,
            peer_enode: peer.node_record.str.clone(),
            peer_info: peer.info.clone(),
            bsc_scan_token_url: helpers::get_bsc_token_url(buy_info.token.buy_token_address),
            bsc_scan_liquidity_url: helpers::get_bsc_tx_url(buy_info.hash),
            peer_city: peer_location_info.city,
            peer_country: peer_location_info.country_code_iso3,
            peer_server: peer_location_info.org,
            start_wallet,
            end_wallet,

            ..Default::default()
        }
    }
}
impl Default for LogToSheets {
    fn default() -> Self {
        Self {
            server_index: 1,
            token_address: "N/A".into(),
            liquidity_hash: "N/A".into(),
            is_server_important: true,
            liq_received_at: "N/A".into(),
            liq_received_at_unix: 0,
            was_tx_direct: false,
            batch_num: 1,
            our_country: "N/A".into(),
            our_city: "N/A".into(),
            our_td: 0,
            peer_td: 0,
            peer_country: "N/A".into(),
            peer_city: "N/A".into(),
            peer_server: "N/A".into(),
            peer_enode: "N/A".into(),
            peer_info: "N/A".into(),
            start_wallet: "N/A".into(),
            end_wallet: "N/A".into(),
            name: "N/A".into(),
            bsc_scan_token_url: "N/A".into(),
            bsc_scan_liquidity_url: "N/A".into(),
        }
    }
}

pub async fn write_data_to_sheets(log_info: LogToSheets) -> anyhow::Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(log_info.server_index as u64)).await;
    let sheets_client = get_client().await?;
    let range = format!("Sheet{}!A:A", log_info.server_index);

    let value_range = ValueRange {
        values: Some(vec![vec![
            json!(log_info.token_address),
            json!(log_info.liquidity_hash),
            json!(log_info.server_index),
            json!(log_info.name),
            json!(log_info.is_server_important),
            json!(log_info.liq_received_at),
            json!(log_info.liq_received_at_unix),
            json!(log_info.was_tx_direct),
            json!(log_info.batch_num),
            json!(log_info.our_country),
            json!(log_info.our_city),
            json!(log_info.our_td),
            json!(log_info.peer_td),
            json!(log_info.peer_country),
            json!(log_info.peer_city),
            json!(log_info.peer_server),
            json!(log_info.peer_enode),
            json!(log_info.peer_info),
            json!(log_info.start_wallet),
            json!(log_info.end_wallet),
            json!(log_info.bsc_scan_token_url),
            json!(log_info.bsc_scan_liquidity_url),
        ]]),
        ..Default::default()
    };

    match sheets_client
        .spreadsheets()
        .values_append(value_range, SPREADSHEET_ID, &range)
        .value_input_option("USER_ENTERED")
        .insert_data_option("INSERT_ROWS")
        .doit()
        .await
    {
        Err(e) => println!("Error: {}", e),
        _ => {}
    }

    println!(
        "Writing to sheets finished at: {}",
        chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S:%f")
            .to_string()
    );
    Ok(())
}

pub async fn get_client() -> anyhow::Result<Sheets<HttpsConnector<HttpConnector>>> {
    let scopes = vec![
        "https://www.googleapis.com/auth/drive",
        "https://www.googleapis.com/auth/drive.file",
        "https://www.googleapis.com/auth/drive.readonly",
        "https://www.googleapis.com/auth/spreadsheets",
        "https://www.googleapis.com/auth/spreadsheets.readonly",
    ];
    let secret = get_secret();
    let auth = ServiceAccountAuthenticator::builder(secret).build().await?;
    let _ = auth.token(&scopes).await?;

    Ok(Sheets::new(
        Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build(),
        ),
        auth,
    ))
}

async fn get_first_empty_row(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    range: &str,
) -> Option<usize> {
    match hub
        .spreadsheets()
        .values_get(SPREADSHEET_ID, &range)
        .doit()
        .await
    {
        Ok((_, value_range)) => {
            if let Some(values) = value_range.values {
                Some(values.len() + 1)
            } else {
                Some(1)
            }
        }
        Err(_) => None,
    }
}

fn get_secret() -> ServiceAccountKey {
    ServiceAccountKey {
        key_type:Some( "service_account".into()),
        private_key_id: Some("92c8db4b9ef724bae2c876e07c166d9bba2eb734".into()),
        client_id: Some("115940951748583473356".into()),
        client_email:"nikola@stellar-utility-379116.iam.gserviceaccount.com".into(),
        token_uri: "https://oauth2.googleapis.com/token".into(),
        auth_uri: Some("https://accounts.google.com/o/oauth2/auth".into()),
        project_id: Some("stellar-utility-379116".into()),
        auth_provider_x509_cert_url: Some("https://www.googleapis.com/oauth2/v1/certs".into()),
        client_x509_cert_url: Some(
            "https://www.googleapis.com/robot/v1/metadata/x509/nikola%40stellar-utility-379116.iam.gserviceaccount.com".into(),
        ),
        private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC6m33qIR+XuIUM\nM4MhKkE/lOrE4s9MLcAdHyBVJymA+aKpBOu028LzB+k8oBtAQ6fMMcz6vDbRABUM\nDzcb+ahZ45ji2PkDATIVNY2fDG47lnJbxZ3xg2Wn/P46MSU8WYXHFbqUR6jVFWXp\nBhUJX+O3HBCYK5NbFtABwCY3hbzt/Ncy1znkBiWK9WN8l3LMywwqFkXW9jDSzp06\nCmR/bhImVSQxkgeNhhY32/IwFDm73446VEid/eZ2wlWf8XhHYUbXl9e087QAHvaB\nCVH8tNbTzzBPqJ2FcHW4biXLXyNwUPFY1bbSeLo9jViqkDX+r/WD5IGs7sfYUhB7\n8aXGC9KFAgMBAAECggEABYUE5/B74T59fPtnFQuNa4aJnTJCRHQT+yiJCcvDQAPi\nSlKRcEOR1CN3RCpONAvsQi906zO3AV6ZwMYQcLzlPGdthcQ6NVsLMrJnUn2JIy0T\ni+BgCB1FW/8xO+JpQgw510YuwyUNeuQLpCVgaOsTrr5fRUkArlCR7YNT/g9wI6/q\ndTT6UwPagcYr322H5kenlPTG+7cGecXMpGw/s2iYeROqieA9XL7UT/GbMHMETHIO\nVCIJYgxNzjVpESzHXwMncuiJd6Sie0wuv2EJgRQF7T/TAmMX3sTo/O1uzFuhvc1T\nn2fj/L+nmFwzic2DzJN67+CISuzx8qyF5ODD8SWdQQKBgQDdpoCO99eL9f56F7mM\n5gcQE9Cd9DAjtzaPrra2tdRqY74vmcg+HDC1EDHoDQCDnHnaMbiaG0Pc66otTy4G\nnDDXs6GULGSgGA8YCNIrsNaM5Tp/rteG1bjv+G3pDZduAqf5q3G30LPccE+Y9cM9\ncW+oBwfWiQJjYw/U8oipmNgG5QKBgQDXhrwFL+fg4L5vr9L1tStoGSFH8dPTjlGL\nK7qrhYy6nz65Odsv/Wy/JW4Q/zH8lQRFq/w3UITWCbZUeoPZwqBtSItxZpsULfdu\nijRCeeNOc7smy3V4bfrGBOGRwAjNNULUDmM3xlKnhdK6F9bcrMbe+L03MklhRdpK\nq7mQ44FDIQKBgQDV0ie2w9SFylshgP2YtNcfZV4c4lIGQlo6JbtRavttXqc72EhZ\n0mwSX3sldlWGoU7TdJ+22pKO7jEO4JFwAwEDNOCsxl6UKmF1OB031LJE3WWfgxWb\nl1V++dNdvaTVlW5h5kgfoQ/Bmf7PelZMUb/7Aj1HcoiBRDEjpoy7vxy3GQKBgDx4\nnUCHVHQQGt6TYol2L5uhkWjyPRDamZ6GwnVlnzqte5fU1977KAvpoJw8PfY0iWJT\nAw0yFlNHnlTNmzj6FrES7az/sPtUelwVgtwz/scASb50z5zensH4lKGkU9Pf4cRF\n1SjNCFvgfGOiVLLN926QM+bMwTH9u2XAEOzKKHaBAoGBAMRvAiiqll0W6qRnT4qI\nOw/AG4RKI7srvuQKPzJLdVRKfRYK53udvtd2wqAe8VWCdkocnjn/sl8spgvh5H0l\n+U6L6ap5imyGB1xBhb3JjjGJDbWN+6RP17UBjZc6nnebeWGuX5ClT0qDt/rKdN6b\nwkDVfqoKf9Keno6LMtZXVsST\n-----END PRIVATE KEY-----\n".into(),
    }
}
