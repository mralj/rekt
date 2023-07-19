use std::time::Instant;

use open_fastrlp::Decodable;
use tracing_subscriber::fmt::time;

use crate::types::hash::H256;
use crate::types::message::Message;

use super::types::errors::ETHError;
use super::types::transaction::{decode_txs, TransactionRequest, ANNO_TX_HASHES};

pub struct TxCache {
    pub(crate) req_count: u8,
    pub(crate) done: bool,
}

pub static mut SUM: u128 = 0;
pub static mut CNT: u128 = 0;
pub static mut MIN: u128 = u128::MAX;
pub static mut MAX: u128 = u128::MIN;

pub fn handle_eth_message(msg: Message) -> Result<Option<Message>, ETHError> {
    match msg.id {
        Some(24) => handle_tx_hashes(msg),
        Some(18) => handle_txs(msg, true),
        Some(26) => handle_txs(msg, false),
        _ => Ok(None),
    }
}

fn handle_tx_hashes(msg: Message) -> Result<Option<Message>, ETHError> {
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

    let anno_hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;
    let mut hashes: Vec<H256> = Vec::with_capacity(anno_hashes.len());

    for h in anno_hashes {
        let cached_tx = ANNO_TX_HASHES.get_mut(&h);
        match cached_tx {
            None => {
                ANNO_TX_HASHES.insert(h, 1);
                hashes.push(h);
            }
            Some(mut v) => {
                if *v > 2 {
                    continue;
                }
                *v += 1;
                hashes.push(h)
            }
        }
    }

    // let hashes: Vec<&H256> = anno_hashes
    //     .iter()
    //     .filter(|h| !TX_HASHES.contains_key(h))
    //     .take(1_000)
    //     .collect();
    unsafe {
        let d = (Instant::now() - msg.received_at).as_nanos();
        SUM += d;
        CNT += 1;
        MIN = if d < MIN { d } else { MIN };
        MAX = if d > MAX { d } else { MAX };
    }

    if hashes.is_empty() {
        return Ok(None);
    }

    Ok(Some(Message {
        id: Some(25),
        kind: Some(crate::types::message::MessageKind::ETH),
        data: TransactionRequest::new(hashes).rlp_encode(),
        received_at: std::time::Instant::now(),
    }))
}

fn handle_txs(msg: Message, is_direct: bool) -> Result<Option<Message>, ETHError> {
    decode_txs(&mut &msg.data[..], is_direct);
    Ok(None)
}
