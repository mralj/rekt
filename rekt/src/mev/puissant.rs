use std::fmt::{Display, Formatter};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    utils::wei_gwei_converter::gwei_to_wei,
    wallets::local_wallets::{generate_mev_bid, PRIORITY_WALLET},
};

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

pub async fn send_mev(
    id: u64,
    bid_gas_price_in_gwei: u64,
    ttl: u64,
    target_tx: Bytes,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let bid = generate_mev_bid(bid_gas_price_in_gwei).await;

    let priority_wallet = &mut PRIORITY_WALLET.write().await;
    priority_wallet.update_nonce_locally();
    let tx = priority_wallet
        .generate_and_sign_buy_tx(gwei_to_wei(3))
        .await
        .expect("Failed to generate and sign priority tx");

    let txs = [
        format!("0x{}", hex::encode(bid)),
        format!("0x{}", hex::encode(&target_tx)),
        format!("0x{}", hex::encode(&tx)),
    ];

    let data = json!({
        "id": id,
        "jsonrpc": "2.0",
        "method": "eth_sendPuissant",
        "params": [{
            "txs": txs,
            "maxTimestamp": chrono::Utc::now().timestamp() as u64 + ttl,
            "acceptReverting": [],
        }]
    });

    let response = client
        .post(PUISSANT_API_URL)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?;

    let response = response.text().await?;
    println!("Puissant response: {}", response);
    Ok(())
}

pub async fn get_mev_status(id: &str) -> anyhow::Result<MevStatusResponse> {
    let url = format!("{}/puissant/{}", PUISSANT_EXPLORER_URL, id);
    let response = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(e) => {
            println!("Puissant get_mev_status err: {}", e);
            anyhow::bail!("Puissant get_mev_status err: {}", e);
        }
    };

    let response = response.json::<MevStatusResponse>().await?;
    Ok(response)
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
    #[serde(rename = "jsonrpc")]
    pub json_rpc: String,
    pub id: u64,
    pub result: String,
}

impl Display for ApiResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.id, self.result)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MevStatusResponse {
    message: String,
    status: u16,
    #[serde(rename = "value")]
    result: MevStatusResult,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MevStatusResult {
    #[serde(rename = "uuid")]
    id: String,
    block: String,
    validator: String,
    status: String,
    info: String,
    txs: Vec<MevStatusTx>,
    created: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MevStatusTx {
    #[serde(rename = "tx_hash")]
    hash: String,
    status: String,
    revert_msg: String,
    accept_revert: bool,
    created: i64,
}

impl Display for MevStatusResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}\n{}", self.status, self.message, self.result)
    }
}

impl Display for MevStatusResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let output = format!(
            "[{}] Block: {},  Validator: {}, Status: {}, Info: {}\n",
            self.id, self.block, self.validator, self.status, self.info
        );

        let mut tx_output: String = "".to_string();
        for tx in &self.txs {
            tx_output.push_str(&format!("{}\n", tx));
        }
        let output = output + &tx_output;

        write!(f, "{}", output)
    }
}

impl Display for MevStatusTx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] Status: {}, Accept revert? {}, Created at: {}",
            self.hash, self.status, self.accept_revert, self.created
        )
    }
}
