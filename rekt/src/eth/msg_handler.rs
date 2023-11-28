use open_fastrlp::Decodable;

use crate::types::hash::{H256, H512};

use super::eth_message::EthMessage;
use super::transactions::decoder::{decode_txs, decode_txs_request, BuyTokenInfo};
use super::transactions::*;
use super::transactions_request::TransactionsRequest;
use super::types::errors::ETHError;
use super::types::protocol::EthProtocol;

pub enum EthMessageHandler {
    Response(EthMessage),
    Buy(BuyTokenInfo),
    None,
}

pub fn handle_eth_message(
    msg: EthMessage,
    peer_hash: H512,
    peer_td: u64,
) -> Result<EthMessageHandler, ETHError> {
    match msg.id {
        EthProtocol::TransactionsMsg => handle_txs(msg),
        EthProtocol::PooledTransactionsMsg => handle_txs(msg),
        EthProtocol::NewPooledTransactionHashesMsg => handle_tx_hashes(msg, peer_hash, peer_td),
        _ => Ok(EthMessageHandler::None),
    }
}

fn handle_tx_hashes(
    msg: EthMessage,
    peer_hash: H512,
    peer_td: u64,
) -> Result<EthMessageHandler, ETHError> {
    //TODO: optimize with custom rlp decoder
    let hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;
    println!(
        "Got {} hashes from {peer_hash}, of td {peer_td}",
        hashes.len()
    );
    if hashes.len() > 300 {
        return Ok(EthMessageHandler::None);
    }

    let hashes_to_request = hashes
        .into_iter()
        .filter(|hash| !cache::was_fetched(hash))
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
        EthProtocol::TransactionsMsg => decode_txs(&mut &msg.data[..], true),
        EthProtocol::PooledTransactionsMsg => decode_txs_request(&mut &msg.data[..]),
        _ => Ok(None),
    };

    if let Ok(Some(buy_info)) = buy_info {
        return Ok(EthMessageHandler::Buy(buy_info));
    }

    Ok(EthMessageHandler::None)
}
