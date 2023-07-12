use bytes::{Buf, Bytes, BytesMut};
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Encodable, Header, HeaderInfo, RlpEncodable};
use sha3::{Digest, Keccak256};

use crate::types::hash::{H160, H256};

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
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
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

        // skip value
        let h = match HeaderInfo::decode(payload_view) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode value header: {:?}", e);
                return Err(e);
            }
        };

        payload_view.advance(h.total_len);

        let _data = match Bytes::decode(payload_view) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode data: {:?}", e);
                return Err(e);
            }
        };

        // println!(
        //     "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
        //     nonce, gas_price, recipient, hash
        // );

        // we skip v, r, s
        buf.advance(tx_header.payload_length);

        Ok(Transaction::default())
    }
}

pub fn decode_txs(buf: &mut &[u8], is_direct: bool) -> Result<Vec<Transaction>, DecodeError> {
    if is_direct {
        decode_txs_direct(buf)
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

        decode_txs_direct(buf)
    }
}

pub fn decode_txs_direct(buf: &mut &[u8]) -> Result<Vec<Transaction>, DecodeError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeError::UnexpectedString);
    }

    let payload_view = &mut &buf[..h.payload_length];
    while !payload_view.is_empty() {
        Transaction::decode(payload_view)?;
    }

    buf.advance(h.payload_length);

    Ok(Vec::new())
}

fn eth_tx_hash(raw_tx: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(raw_tx);
    let result = hasher.finalize();
    to_hex_string(&result)
}

fn to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}
