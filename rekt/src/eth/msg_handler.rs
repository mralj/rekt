use open_fastrlp::Decodable;

use crate::token::tokens_to_buy::there_are_no_tokens_to_buy;
use crate::types::hash::H256;

use super::eth_message::EthMessage;
use super::transactions::cache::{self, CACHE};
use super::transactions::decoder::{decode_txs, decode_txs_request};
use super::transactions_request::TransactionsRequest;
use super::types::errors::ETHError;
use super::types::protocol::EthProtocol;

pub fn handle_eth_message(msg: EthMessage) -> Result<Option<EthMessage>, ETHError> {
    if there_are_no_tokens_to_buy() {
        return Ok(None);
    }
    match msg.id {
        EthProtocol::TransactionsMsg => handle_txs(msg),
        EthProtocol::PooledTransactionsMsg => handle_txs(msg),
        EthProtocol::NewPooledTransactionHashesMsg => handle_tx_hashes(msg),
        _ => Ok(None),
    }
}

fn handle_tx_hashes(msg: EthMessage) -> Result<Option<EthMessage>, ETHError> {
    //NOTE: we can optimize this here is how this works "under the hood":
    //
    // fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
    //     let h = Header::decode(buf)?;
    //     if !h.list {
    //         return Err(DecodeError::UnexpectedString)
    //     }

    //     let payload_view = &mut &buf[..h.payload_length];

    //     let mut to = alloc::vec::Vec::new();
    //     while !payload_view.is_empty() {
    //         to.push(E::decode(payload_view)?);
    //     }

    //     buf.advance(h.payload_length);

    //     Ok(to)
    // }
    // the main issue with this is that we can know in advance how many elements we can allocate
    // for Vec as RLP holds the size of the data, also we could use smth. like smallvec instead of
    // vector to allocate on stack
    // this usually takes couple hundred of `ns` to decode with occasional spikes to 2 <`us`

    let hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;
    let hashes_to_request = hashes
        .into_iter()
        .filter(|hash| !cache::has(hash))
        .take(1_000)
        .collect::<Vec<_>>();

    if hashes_to_request.is_empty() {
        return Ok(None);
    }

    Ok(Some(EthMessage {
        id: EthProtocol::GetPooledTransactionsMsg,
        data: TransactionsRequest::new(hashes_to_request).rlp_encode(),
    }))
}

fn handle_txs(msg: EthMessage) -> Result<Option<EthMessage>, ETHError> {
    let _ = match msg.id {
        EthProtocol::TransactionsMsg => decode_txs(&mut &msg.data[..]),
        EthProtocol::PooledTransactionsMsg => decode_txs_request(&mut &msg.data[..]),
        _ => Ok(()),
    };

    Ok(None)
}
