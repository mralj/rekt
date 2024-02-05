use std::usize;

use bytes::Buf;
use open_fastrlp::{Decodable, Header, HeaderInfo};

use crate::p2p::protocol::ProtocolVersion;
use crate::token::tokens_to_buy::{MAX_SIZE, MIN_SIZE};
use crate::types::hash::H256;

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
    proto_v: ProtocolVersion,
) -> Result<EthMessageHandler, ETHError> {
    match msg.id {
        EthProtocol::TransactionsMsg => handle_txs(msg),
        EthProtocol::PooledTransactionsMsg => handle_txs(msg),
        EthProtocol::NewPooledTransactionHashesMsg => handle_tx_hashes(msg, proto_v),
        _ => Ok(EthMessageHandler::None),
    }
}

fn handle_tx_hashes(
    msg: EthMessage,
    proto_v: ProtocolVersion,
) -> Result<EthMessageHandler, ETHError> {
    if proto_v < ProtocolVersion::Eth68 {
        handle_tx_hashes_before_eth_68(msg)
    } else {
        handle_tx_hashes_after_eth_68(msg)
    }
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

fn handle_tx_hashes_before_eth_68(msg: EthMessage) -> Result<EthMessageHandler, ETHError> {
    //TODO: optimize with custom rlp decoder
    let hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;

    let hashes_to_request = hashes
        .into_iter()
        .filter(|hash| cache::mark_as_requested(hash) == cache::TxCacheStatus::NotRequested)
        .collect::<Vec<_>>();

    if hashes_to_request.is_empty() {
        return Ok(EthMessageHandler::None);
    }

    if hashes_to_request.len() > 500 {
        return Ok(EthMessageHandler::None);
    }

    Ok(EthMessageHandler::Response(EthMessage::new(
        EthProtocol::GetPooledTransactionsMsg,
        TransactionsRequest::new(hashes_to_request).rlp_encode(),
    )))
}

fn handle_tx_hashes_after_eth_68(msg: EthMessage) -> Result<EthMessageHandler, ETHError> {
    let buf = &mut &msg.data[..];
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(ETHError::RLPDecoding(
            open_fastrlp::DecodeError::UnexpectedString,
        ));
    }

    // skip the collection of transaction types
    HeaderInfo::skip_next_item(buf)?;

    let metadata = Header::decode(buf)?;
    if !metadata.list {
        return Err(ETHError::RLPDecoding(
            open_fastrlp::DecodeError::UnexpectedString,
        ));
    }

    let payload_view = &mut &buf[..metadata.payload_length];
    let mut sizes = Vec::with_capacity(4_000);
    while !payload_view.is_empty() {
        sizes.push(usize::decode(payload_view)?);
    }

    buf.advance(metadata.payload_length);
    let metadata = Header::decode(buf)?;

    if !metadata.list {
        return Err(ETHError::RLPDecoding(
            open_fastrlp::DecodeError::UnexpectedString,
        ));
    }

    let mut hashes = Vec::with_capacity(sizes.len());
    let payload_view = &mut &buf[..metadata.payload_length];
    let mut i = 0;
    while !payload_view.is_empty() {
        let hash = H256::decode(payload_view)?;
        if !tx_size_is_valid(sizes[i]) {
            cache::mark_as_fetched(&hash);
            continue;
        }

        if cache::mark_as_requested(&hash) == cache::TxCacheStatus::NotRequested {
            hashes.push(hash);
        }
        i += 1;
    }

    if hashes.is_empty() {
        return Ok(EthMessageHandler::None);
    }

    if hashes.len() > 500 {
        return Ok(EthMessageHandler::None);
    }

    Ok(EthMessageHandler::Response(EthMessage::new(
        EthProtocol::GetPooledTransactionsMsg,
        TransactionsRequest::new(hashes).rlp_encode(),
    )))
}

fn tx_size_is_valid(size: usize) -> bool {
    unsafe {
        if MIN_SIZE == 0 && MAX_SIZE == 0 {
            return true;
        }

        size >= MIN_SIZE && size <= MAX_SIZE
    }
}
