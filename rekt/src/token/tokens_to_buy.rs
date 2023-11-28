use dashmap::DashSet;
use futures::StreamExt;
use once_cell::sync::Lazy;
use tokio::time::interval;

use crate::{p2p::peer::BUY_IS_IN_PROGRESS, utils::helpers::get_bsc_token_url};

use super::token::{Token, TokenAddress};

const TOKENS_TO_BUY_FILE_PATH: &str = "tokens_to_buy.json";
const REFRESH_TOKENS_INTERVAL: u64 = 10;

pub static mut TOKENS_TO_BUY: Vec<Token> = Vec::new();
pub static mut MIN_NONCE: u64 = 0;
pub static mut MAX_NONCE: u64 = 0;
pub static mut PCS_LIQ: bool = false;

pub static BOUGHT_TOKENS: Lazy<DashSet<TokenAddress>> = Lazy::new(|| DashSet::new());

pub fn import_tokens_to_buy() {
    unsafe {
        TOKENS_TO_BUY.reserve(10);
    }
    tokio::task::spawn(async move {
        let mut read_tokens_ticker = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(REFRESH_TOKENS_INTERVAL),
        ));

        while read_tokens_ticker.next().await.is_some() {
            unsafe {
                if BUY_IS_IN_PROGRESS {
                    continue;
                }
            }

            if let Ok(tokens) = read_tokens_to_buy_from_file().await {
                for mut token in tokens {
                    if BOUGHT_TOKENS.contains(&token.buy_token_address) {
                        continue;
                    }
                    unsafe {
                        // If a newer version of the token is in the file,
                        // update the in-memory list by removing and re-adding the token.
                        if let Some(token_index) = TOKENS_TO_BUY
                            .iter()
                            .position(|t| t.buy_token_address == token.buy_token_address)
                        {
                            if TOKENS_TO_BUY[token_index].version < token.version {
                                TOKENS_TO_BUY.swap_remove(token_index);
                            } else {
                                continue;
                            }
                        }

                        token.prepare_buy_txs_for_gas_price_range().await;
                        println!(
                            "Added token to buy: {}",
                            get_bsc_token_url(token.buy_token_address)
                        );
                        TOKENS_TO_BUY.push(token);
                        update_global_liq_setting();
                    }
                }
            } else {
                println!("Error while reading tokens to buy from file");
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
            TOKENS_TO_BUY.swap_remove(index);
        }
    }
    update_global_liq_setting();
}

#[inline(always)]
pub fn get_token_to_buy(address: &TokenAddress, nonce: u64) -> Option<(&Token, usize)> {
    unsafe {
        let idx = TOKENS_TO_BUY
            .iter()
            .position(|v| &v.enable_buy_config.tx_to == address)?;

        let token = &TOKENS_TO_BUY[idx];
        if let Some(from) = &token.from {
            if nonce < from.min_nonce || nonce > from.max_nonce {
                return None;
            }
        }

        Some((&TOKENS_TO_BUY[idx], idx))
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
pub fn get_token_to_buy_by_address(address: &TokenAddress) -> Option<Token> {
    unsafe {
        let idx = TOKENS_TO_BUY
            .iter()
            .position(|v| &v.buy_token_address == address)?;

        Some(TOKENS_TO_BUY.swap_remove(idx))
    }
}

#[inline(always)]
pub fn tx_is_enable_buy(
    token: &Token,
    index_of_token_in_buy_list: usize,
    tx_data: &[u8],
) -> Option<Token> {
    if !tx_data.starts_with(token.enable_buy_config.enable_buy_tx_hash.as_ref()) {
        return None;
    }

    if !token.trade_status_is_enable(tx_data) {
        return None;
    }

    unsafe { Some(TOKENS_TO_BUY.swap_remove(index_of_token_in_buy_list)) }
}

#[inline(always)]
pub fn tx_nonce_is_ok(nonce: u64) -> bool {
    unsafe {
        if MAX_NONCE == 0 {
            return true;
        }
        nonce >= MIN_NONCE && nonce <= MAX_NONCE
    }
}

pub fn remove_all_tokens_to_buy() {
    unsafe {
        TOKENS_TO_BUY.clear();
        update_global_liq_setting();
    }
}

fn update_global_liq_setting() {
    unsafe {
        MIN_NONCE = 0;
        MAX_NONCE = 0;
        PCS_LIQ = false;
        for token in TOKENS_TO_BUY.iter() {
            if token.liq_will_be_added_via_pcs {
                PCS_LIQ = true;
            }
            if let Some(from) = &token.from {
                if MIN_NONCE > from.min_nonce {
                    MIN_NONCE = from.min_nonce;
                }
                if MAX_NONCE < from.max_nonce {
                    MAX_NONCE = from.max_nonce;
                }
            }
        }
    }
}
async fn read_tokens_to_buy_from_file() -> Result<Vec<Token>, std::io::Error> {
    let tokens_to_buy_file = tokio::fs::read_to_string(TOKENS_TO_BUY_FILE_PATH).await?;
    let tokens_to_buy: Vec<Token> = serde_json::from_str(&tokens_to_buy_file)?;
    Ok(tokens_to_buy)
}
