use dashmap::DashSet;
use futures::StreamExt;
use once_cell::sync::Lazy;
use tokio::time::interval;

use super::token::{Token, TokenAddress};

const TOKENS_TO_BUY_FILE_PATH: &str = "tokens_to_buy.json";
const REFRESH_TOKENS_INTERVAL: u64 = 20;

pub static mut TOKENS_TO_BUY: Vec<Token> = Vec::new();
pub static BOUGHT_TOKENS: Lazy<DashSet<TokenAddress>> = Lazy::new(|| DashSet::new());

pub fn import_tokens_to_buy() {
    unsafe {
        TOKENS_TO_BUY.reserve(10);
    }
    tokio::task::spawn(async move {
        let mut read_tokens_ticker = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(REFRESH_TOKENS_INTERVAL),
        ));

        while let Some(_) = read_tokens_ticker.next().await {
            match read_tokens_to_buy_from_file().await {
                Ok(tokens) => {
                    for mut token in tokens {
                        if BOUGHT_TOKENS.contains(&token.buy_token_address) {
                            continue;
                        }
                        unsafe {
                            if TOKENS_TO_BUY
                                .iter()
                                .any(|v| v.buy_token_address == token.buy_token_address)
                            {
                                continue;
                            }
                            token.prepare_buy_txs_per_gas_price().await;
                            println!("Added token to buy: {}", token.buy_token_address);
                            TOKENS_TO_BUY.push(token);
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading tokens to buy from file: {}", e);
                }
            }
        }
    });
}

#[inline(always)]
pub fn there_are_no_tokens_to_buy() -> bool {
    unsafe { TOKENS_TO_BUY.is_empty() }
}

pub fn mark_token_as_bought(buy_token_address: TokenAddress) {
    BOUGHT_TOKENS.insert(buy_token_address);
    unsafe {
        if let Some(index) = TOKENS_TO_BUY
            .iter()
            .position(|v| v.buy_token_address == buy_token_address)
        {
            TOKENS_TO_BUY.remove(index);
        }
    }
}
#[inline(always)]
pub fn get_token_to_buy(address: &TokenAddress) -> Option<Token> {
    unsafe {
        let idx = TOKENS_TO_BUY
            .iter()
            .position(|v| &v.enable_buy_config.tx_to == address)?;

        Some(TOKENS_TO_BUY.remove(idx))
    }
}

#[inline(always)]
pub fn get_token(address: &TokenAddress) -> Option<&Token> {
    unsafe {
        TOKENS_TO_BUY
            .iter()
            .find(|v| &v.enable_buy_config.tx_to == address)
    }
}

#[inline(always)]
pub fn get_token_by_address(address: &TokenAddress) -> Option<&Token> {
    unsafe {
        TOKENS_TO_BUY
            .iter()
            .find(|v| &v.buy_token_address == address)
    }
}

#[inline(always)]
pub fn tx_is_enable_buy(token: &Token, tx_data: &[u8]) -> bool {
    tx_data.starts_with(token.enable_buy_config.enable_buy_tx_hash.as_ref())
}

async fn read_tokens_to_buy_from_file() -> Result<Vec<Token>, std::io::Error> {
    let tokens_to_buy_file = tokio::fs::read_to_string(TOKENS_TO_BUY_FILE_PATH).await?;
    let tokens_to_buy: Vec<Token> = serde_json::from_str(&tokens_to_buy_file)?;
    Ok(tokens_to_buy)
}
