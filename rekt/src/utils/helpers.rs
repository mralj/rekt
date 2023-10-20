use ethers::types::H160;

use crate::types::hash::H256;

pub fn get_bsc_token_url(token_address: H160) -> String {
    format!("https://bscscan.com/token/{:#x}", token_address)
}

pub fn get_bsc_tx_url(tx_hash: H256) -> String {
    format!("https://bscscan.com/tx/{:#x}", tx_hash)
}
