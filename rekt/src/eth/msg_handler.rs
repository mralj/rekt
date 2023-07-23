use std::time::Instant;

use open_fastrlp::Decodable;
use tracing_subscriber::fmt::time;

use crate::types::hash::{H256, H512};
use crate::types::message::Message;

use super::types::errors::ETHError;
use super::types::transaction::{decode_txs, TransactionRequest, ANNO_TX_HASHES};

pub struct TxCache {
    pub(crate) req_count: u8,
    pub(crate) done: bool,
}

pub static mut SUM: u128 = 0;
pub static mut SUM_CNT: u128 = 0;
pub static mut SUM_BYTE: u128 = 0;

pub static mut CNT: u128 = 0;
pub static mut MIN: u128 = u128::MAX;
pub static mut MAX: u128 = u128::MIN;
pub static mut MAX_ID: u64 = 0;
pub static mut MAX_CNT: u128 = u128::MIN;
pub static mut MAX_CNT_ID: u64 = 0;
pub static mut MAX_BYTE: usize = usize::MIN;
pub static mut MAX_BYTE_ID: u64 = 0;
pub static mut IS_DIRECT: bool = false;

pub static mut L_1: usize = 0;
pub static mut L_1_10: usize = 0;
pub static mut L_10_20: usize = 0;
pub static mut L_20_50: usize = 0;
pub static mut L_50_100: usize = 0;
pub static mut L_100_300: usize = 0;
pub static mut L_300_500: usize = 0;
pub static mut L_500_1000: usize = 0;
pub static mut L_1000_1500: usize = 0;
pub static mut L_1500_2000: usize = 0;
pub static mut L_2000: usize = 0;

pub fn handle_eth_message(
    msg: Message,
    connected_to_peer_since: Instant,
    peer_id: H512,
) -> Result<Option<Message>, ETHError> {
    match msg.id {
        Some(24) => handle_tx_hashes(msg, connected_to_peer_since, peer_id),
        Some(18) => handle_txs(msg, true),
        Some(26) => handle_txs(msg, false),
        _ => Ok(None),
    }
}

fn handle_tx_hashes(
    msg: Message,
    connected_to_peer_since: Instant,
    peer_id: H512,
) -> Result<Option<Message>, ETHError> {
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
    let anno_len = anno_hashes.len();

    if anno_len >= 1_000 {
        tracing::info!(
            "LEN: {}, connected for: {}, ID: {}",
            anno_len,
            Instant::now()
                .duration_since(connected_to_peer_since)
                .as_secs(),
            peer_id
        );
    }

    unsafe {
        match anno_len {
            1 => L_1 += 1,
            2..=10 => L_1_10 += 1,
            11..=20 => L_10_20 += 1,
            21..=50 => L_20_50 += 1,
            51..=100 => L_50_100 += 1,
            101..=300 => L_100_300 += 1,
            301..=500 => L_300_500 += 1,
            501..=1000 => L_500_1000 += 1,
            1001..=1500 => L_1000_1500 += 1,
            1501..=2000 => L_1500_2000 += 1,
            _ => L_2000 += 1,
        }
    }

    let mut hashes: Vec<H256> = Vec::with_capacity(std::cmp::min(anno_hashes.len(), 1_001));

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
    if hashes.is_empty() {
        return Ok(None);
    }

    Ok(Some(Message {
        req_id: 0,
        id: Some(25),
        kind: Some(crate::types::message::MessageKind::ETH),
        data: TransactionRequest::new(hashes).rlp_encode(),
        received_at: std::time::Instant::now(),
    }))
}

fn handle_txs(msg: Message, is_direct: bool) -> Result<Option<Message>, ETHError> {
    decode_txs(&mut &msg.data[..], is_direct, msg.received_at, msg.req_id);
    Ok(None)
}
