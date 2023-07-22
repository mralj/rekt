use std::hash::{BuildHasher, Hasher};
use std::str::FromStr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::{Buf, Bytes, BytesMut};
use dashmap::DashMap;
use ethers::types::{U128, U256};
use once_cell::sync::Lazy;
use open_fastrlp::{Decodable, DecodeError, Encodable, Header, HeaderInfo, RlpEncodable};
use sha3::{Digest, Keccak256};

use crate::eth::msg_handler::{
    CNT, IS_DIRECT, MAX, MAX_BYTE, MAX_BYTE_ID, MAX_CNT, MAX_CNT_ID, MAX_ID, MIN, SUM,
};
use crate::types::hash::{H160, H256};

#[derive(Default)]
pub struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // Take the first 8 bytes from the input and interpret them as a u64.
        // This will panic if bytes is less than 8 bytes long.
        let bytes: [u8; 8] = bytes[..8]
            .try_into()
            .expect("Should've had at least 8 bytes");
        self.0 = u64::from_ne_bytes(bytes);
    }
}

#[derive(Default, Clone)]
pub struct IdentityBuildHasher;

impl BuildHasher for IdentityBuildHasher {
    type Hasher = IdentityHasher;

    fn build_hasher(&self) -> Self::Hasher {
        IdentityHasher(0)
    }
}

pub static ANNO_TX_HASHES: Lazy<DashMap<H256, u8, IdentityBuildHasher>> = Lazy::new(|| {
    DashMap::with_capacity_and_hasher_and_shard_amount(
        4_000_000,
        IdentityBuildHasher::default(),
        256,
    )
});

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable)]
pub struct TransactionRequest {
    id: u64,
    hashes: Vec<H256>,
}

impl TransactionRequest {
    pub fn new(hashes: Vec<H256>) -> Self {
        Self { id: 0, hashes }
    }

    pub fn rlp_encode(&self) -> BytesMut {
        let mut rlp = BytesMut::new();
        self.encode(&mut rlp);
        rlp
    }
}

// Nonce
// Gas Price
// Gas Limit
// Recipient Address
// Value
// Data
// v
// r
// s
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable)]
pub struct Transaction {
    pub nonce: U256,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub to: H160,
    pub value: U256,
    pub data: Bytes,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            nonce: U256::zero(),
            gas_price: U128::zero(),
            gas_limit: U256::zero(),
            to: H160::zero(),
            value: U256::zero(),
            data: Bytes::new(),
        }
    }
}

impl Transaction {
    fn decode(
        buf: &mut &[u8],
        msg_received_at: Instant,
        id: u64,
        is_direct: bool,
    ) -> Result<H256, DecodeError> {
        let tx_header_info = HeaderInfo::decode(buf)?;
        let hash = eth_tx_hash(&buf[..tx_header_info.total_len]);

        let tx_header = match Header::decode_from_info(buf, tx_header_info) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode header: {:?}", e);
                return Err(e);
            }
        };

        if !tx_header.list {
            return Err(DecodeError::UnexpectedString);
        }

        if let Some(t_c) = ANNO_TX_HASHES.insert(hash, u8::MAX) {
            if t_c == u8::MAX {
                buf.advance(tx_header.payload_length);
                return Err(DecodeError::Custom("Already decoded"));
            }
        }

        let payload_view = &mut &buf[..tx_header.payload_length];

        let nonce = match u64::decode(payload_view) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode nonce: {:?}", e);
                return Err(e);
            }
        };

        let gas_price = match u64::decode(payload_view) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode gas price: {:?}", e);
                return Err(e);
            }
        };

        // skip gas limit
        let h = match HeaderInfo::decode(payload_view) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode gas price header: {:?}", e);
                return Err(e);
            }
        };

        payload_view.advance(h.total_len);

        let recipient = H160::decode(payload_view)?;

        unsafe {
            let d = (Instant::now().duration_since(msg_received_at)).as_micros();
            SUM += d;
            CNT += 1;
            MIN = if d < MIN { d } else { MIN };
            MAX = if d > MAX {
                MAX_ID = id;
                d
            } else {
                MAX
            };
        }

        // if recipient == H160::from_str("0x13f4EA83D0bd40E75C8222255bc855a974568Dd4").unwrap() {
        //     let timestamp = SystemTime::now()
        //         .duration_since(UNIX_EPOCH)
        //         .unwrap()
        //         .as_micros();
        //     tracing::info!("{},{:#x}", timestamp, hash);
        // }

        // skip value
        // let h = match HeaderInfo::decode(payload_view) {
        //     Ok(h) => h,
        //     Err(e) => {
        //         println!("Failed to decode value header: {:?}", e);
        //         return Err(e);
        //     }
        // };

        // payload_view.advance(h.total_len);

        // let _data = match Bytes::decode(payload_view) {
        //     Ok(n) => n,
        //     Err(e) => {
        //         println!("Failed to decode data: {:?}", e);
        //         return Err(e);
        //     }
        // };

        // println!(
        //     "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
        //     nonce, gas_price, recipient, hash
        // );

        // we skip v, r, s
        buf.advance(tx_header.payload_length);

        Ok(hash)
    }
}

pub fn decode_txs(
    buf: &mut &[u8],
    is_direct: bool,
    msg_received_at: Instant,
    id: u64,
) -> Result<Vec<Transaction>, DecodeError> {
    if is_direct {
        decode_txs_direct(buf, msg_received_at, id, is_direct)
    } else {
        let h = Header::decode(buf)?;
        if !h.list {
            return Err(DecodeError::UnexpectedString);
        }
        // skip decoding request id
        let h = match HeaderInfo::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode request id header: {:?}", e);
                return Err(e);
            }
        };

        buf.advance(h.total_len);

        decode_txs_direct(buf, msg_received_at, id, is_direct)
    }
}

pub fn decode_txs_direct(
    buf: &mut &[u8],
    msg_received_at: Instant,
    id: u64,
    is_direct: bool,
) -> Result<Vec<Transaction>, DecodeError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeError::UnexpectedString);
    }

    let payload_view = &mut &buf[..h.payload_length];
    let mut cnt = 0;
    while !payload_view.is_empty() {
        Transaction::decode(payload_view, msg_received_at, id, is_direct)?;
        cnt += 1;
    }

    unsafe {
        MAX_CNT = if cnt > MAX_CNT {
            MAX_CNT_ID = id;
            cnt
        } else {
            MAX_CNT
        };
        MAX_BYTE = if buf.len() > MAX_BYTE {
            MAX_BYTE_ID = id;
            IS_DIRECT = is_direct;
            buf.len()
        } else {
            MAX_BYTE
        };
    }

    // for h in hashes {
    //     TX_HASHES.insert(h);
    // }

    buf.advance(h.payload_length);

    Ok(Vec::new())
}

fn eth_tx_hash(raw_tx: &[u8]) -> H256 {
    let mut hasher = Keccak256::new();
    hasher.update(raw_tx);
    let result = hasher.finalize();
    H256::from_slice(&result)
}
