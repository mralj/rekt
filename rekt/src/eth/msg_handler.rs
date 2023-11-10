use open_fastrlp::Decodable;
use tokio::time::interval;
use tokio_stream::StreamExt;

use crate::types::hash::H256;

use super::eth_message::EthMessage;
use super::transactions::decoder::{
    decode_txs, decode_txs_request, BuyTokenInfo, TxDecodingResult,
};
use super::transactions::*;
use super::transactions_request::TransactionsRequest;
use super::types::errors::ETHError;
use super::types::protocol::EthProtocol;

pub static mut UNDER_10: usize = 0;
pub static mut UNDER_20: usize = 0;
pub static mut UNDER_30: usize = 0;
pub static mut UNDER_50: usize = 0;
pub static mut UNDER_100: usize = 0;
pub static mut UNDER_200: usize = 0;
pub static mut OVER_200: usize = 0;
pub static mut TOTAL: usize = 0;

pub enum EthMessageHandler {
    Response(EthMessage),
    Buy(BuyTokenInfo),
    None,
}

pub fn handle_eth_message(msg: EthMessage) -> Result<EthMessageHandler, ETHError> {
    match msg.id {
        EthProtocol::TransactionsMsg => handle_txs(msg),
        EthProtocol::PooledTransactionsMsg => handle_txs(msg),
        EthProtocol::NewPooledTransactionHashesMsg => handle_tx_hashes(msg),
        _ => Ok(EthMessageHandler::None),
    }
}

fn handle_tx_hashes(msg: EthMessage) -> Result<EthMessageHandler, ETHError> {
    //TODO: optimize with custom rlp decoder
    let hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;
    if hashes.len() > 1_000 {
        return Ok(EthMessageHandler::None);
    }

    let hashes_to_request = hashes
        .into_iter()
        .filter(|hash| {
            let previous_tx_cache_status = cache::mark_as_announced(hash);
            previous_tx_cache_status == cache::TxCacheStatus::NotAnnounced
        })
        .collect::<Vec<_>>();

    if hashes_to_request.is_empty() {
        return Ok(EthMessageHandler::None);
    }

    Ok(EthMessageHandler::Response(EthMessage::new(
        EthProtocol::GetPooledTransactionsMsg,
        TransactionsRequest::new(hashes_to_request).rlp_encode(),
    )))
}

fn handle_txs(msg: EthMessage) -> Result<EthMessageHandler, ETHError> {
    let buy_info = match msg.id {
        EthProtocol::TransactionsMsg => decode_txs(&mut &msg.data[..])?,
        EthProtocol::PooledTransactionsMsg => decode_txs_request(&mut &msg.data[..])?,
        _ => return Ok(EthMessageHandler::None),
    };

    match buy_info {
        TxDecodingResult::Buy(b) => Ok(EthMessageHandler::Buy(b)),
        TxDecodingResult::NoBuy(count) => {
            let elapsed = msg.created_on.elapsed().as_micros();
            // if elapsed >= tokio::time::Duration::from_micros(100) {
            //     println!("[{count}] handling took: {:?}", msg.created_on.elapsed());
            // }
            unsafe {
                TOTAL += 1;
                if elapsed <= 10 {
                    UNDER_10 += 1;
                } else if elapsed <= 20 {
                    UNDER_20 += 1;
                } else if elapsed <= 30 {
                    UNDER_30 += 1;
                } else if elapsed <= 50 {
                    UNDER_50 += 1;
                } else if elapsed <= 100 {
                    UNDER_100 += 1;
                } else if elapsed <= 200 {
                    UNDER_200 += 1;
                } else {
                    OVER_200 += 1;
                }
            }

            Ok(EthMessageHandler::None)
        }
    }
}

pub fn logger() {
    tokio::spawn(async {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(60),
        ));

        let started = tokio::time::Instant::now();

        while let Some(_) = stream.next().await {
            unsafe {
                println!("=== STATS ===");
                println!("Test duration: {:?} min", started.elapsed().as_secs() / 60);
                println!("TOTAL: {}", TOTAL);
                println!(
                    "UNDER_10: {}, {}%",
                    UNDER_10,
                    f64::round(((UNDER_10 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_20: {}, {}%",
                    UNDER_20,
                    f64::round(((UNDER_20 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_30: {}, {}%",
                    UNDER_30,
                    f64::round(((UNDER_30 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_50: {}, {}%",
                    UNDER_50,
                    f64::round(((UNDER_50 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_100: {}, {}%",
                    UNDER_100,
                    f64::round(((UNDER_100 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_200: {}, {}%",
                    UNDER_200,
                    f64::round(((UNDER_200 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "OVER_200: {}, {}%",
                    OVER_200,
                    f64::round(((OVER_200 as f64) * 100.0) / TOTAL as f64)
                );
                println!("=== END ===");
            }
        }
    });
}
