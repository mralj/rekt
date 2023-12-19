use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

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
