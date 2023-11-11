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
        EthProtocol::NewPooledTransactionHashesMsg => {
            // unsafe {
            //     TOTAL += 1;
            // }
            //
            handle_tx_hashes(msg)
        }
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
            let elapsed = msg.created_on.elapsed();
            // if elapsed >= tokio::time::Duration::from_micros(100) {
            //     println!("[{count}] handling took: {:?}", msg.created_on.elapsed());
            // }
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
                println!("=== END ===");
            }
        }
    });
}
