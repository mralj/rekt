use std::fmt::{Display, Formatter};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::wallets::local_wallets::generate_mev_bid;

const PUISSANT_API_URL: &str = "https://puissant-bsc.48.club";
const PUISSANT_EXPLORER_URL: &str = "https://explorer.48.club/api/v1";

pub async fn ping() {
    let url = format!("{}/ping", PUISSANT_EXPLORER_URL);
    match reqwest::get(&url).await {
        Ok(resp) => {
            let resp: PingResponse = resp.json().await.unwrap();
            if resp.status == 200 && resp.message == "pong" {
                println!("Puissant ONLINE: {}", resp);
            } else {
                println!("Puissant OFFLINE: {}", resp);
            }
        }
        Err(e) => {
            println!("Puissant OFFLINE: {}", e);
            return;
        }
    }
}

pub async fn get_score() {
    let url = format!("{}/score", PUISSANT_EXPLORER_URL);
    match reqwest::get(&url).await {
        Ok(resp) => {
            let resp: ScoreResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    println!("Puissant score err: {}", e);
                    return;
                }
            };
            if resp.status == 200 {
                println!("{}", resp);
            } else {
                println!("Puissant score err: {}", resp);
            }
        }
        Err(e) => {
            println!("Puissant score err: {}", e);
            return;
        }
    }
}

pub async fn send_mev(tx: Bytes, id: u64, bid_gas_price_in_gwei: u64) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let bid = generate_mev_bid(bid_gas_price_in_gwei).await;

    let data = json!({
        "id": id,
        "jsonrpc": "2.0",
        "method": "eth_sendPuissant",
        "params": [{
            "txs": [format!("0x{}", hex::encode(bid)) , format!("0x{}", hex::encode(&tx))],
            "maxTimestamp": chrono::Utc::now().timestamp() as u64 + 12,
            "acceptReverting": [],
        }]
    });

    let response = match client
        .post(PUISSANT_API_URL)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("Puissant send_private_tx err: {}", e);
            return Ok(());
        }
    };

    let response = match response.text().await {
        Ok(r) => r,
        Err(e) => {
            println!("Puissant send_private_tx err: {} \n", e);
            return Ok(());
        }
    };

    println!("Response: {:?}", response);

    Ok(())
}

pub async fn send_private_tx(tx: Bytes, id: u64) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let data = json!({
        "id": id,
        "jsonrpc": "2.0",
        "method": "eth_sendPrivateRawTransaction",
        "params": [format!("0x{}", hex::encode(&tx))]
    });

    let response = match client
        .post(PUISSANT_API_URL)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("Puissant send_private_tx err: {}", e);
            return Ok(());
        }
    };

    let response = match response.json::<ApiResponse>().await {
        Ok(r) => r,
        Err(e) => {
            println!("Puissant send_private_tx err: {} \n", e);
            return Ok(());
        }
    };

    println!("Response: {:?}", response);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct PingResponse {
    message: String,
    status: u16,
}

impl Display for PingResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.status, self.message)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ScoreResponse {
    message: String,
    status: u16,
    #[serde(rename = "value")]
    data: ScoreData,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScoreData {
    #[serde(rename = "query")]
    address: String,
    score: u64,
    #[serde(rename = "type")]
    score_type: String,
}

impl Display for ScoreResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] score for {} is {}",
            self.status, self.data.address, self.data.score
        )
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// struct MevArgs {
//     txs: Vec<String>,
//     #[serde(rename = "maxTimestamp")]
//     max_timestamp: u64,
//     #[serde(rename = "acceptReverting")]
//     txs_hashes_which_can_fail: Vec<String>,
// }
//
// impl MevArgs {
//     pub fn new(txs: Vec<String>, txs_hashes_which_can_fail: Vec<String>) -> Self {
//         Self {
//             txs,
//             max_timestamp: chrono::Utc::now().timestamp_millis() as u64 + 1000 * 12,
//             txs_hashes_which_can_fail,
//         }
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    #[serde(rename = "jsonrpc")]
    json_rpc: String,
    id: u64,
    result: String,
}

impl Display for ApiResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.id, self.result)
    }
}
