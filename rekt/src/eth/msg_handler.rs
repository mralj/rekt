use open_fastrlp::Decodable;

use crate::types::hash::H256;
use crate::types::message::Message;

use super::types::errors::ETHError;
use super::types::transaction::{decode_txs, TransactionRequest, TX_HASHES};

pub struct TxCache {
    pub(crate) req_count: u8,
    pub(crate) done: bool,
}

impl TxCache {
    pub(crate) fn new_from_anno() -> Self {
        Self {
            req_count: 1,
            done: false,
        }
    }

    pub(crate) fn new_from_direct() -> Self {
        Self {
            req_count: 0,
            done: true,
        }
    }
}

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
        match TX_HASHES.get_mut(&h) {
            None => {
                TX_HASHES.insert(h, TxCache::new_from_anno());
                hashes.push(h);
            }
            Some(tx) => {
                if tx.done {
                    continue;
                }

                if tx.req_count > 3 {
                    continue;
                }

                TX_HASHES.alter(&h, |k, mut v| {
                    v.req_count += 1;
                    v
                });

                hashes.push(h)
            }
        }
    }

    // let hashes: Vec<&H256> = anno_hashes
    //     .iter()
    //     .filter(|h| !TX_HASHES.contains_key(h))
    //     .take(1_000)
    //     .collect();

    if hashes.is_empty() {
        return Ok(None);
    }

    Ok(Some(Message {
        id: Some(25),
        kind: Some(crate::types::message::MessageKind::ETH),
        data: TransactionRequest::new(hashes).rlp_encode(),
    }))
}

fn handle_txs(msg: Message, is_direct: bool) -> Result<Option<Message>, ETHError> {
    decode_txs(&mut &msg.data[..], is_direct);
    Ok(None)
}
