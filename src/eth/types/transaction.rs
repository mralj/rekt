use bytes::{Buf, Bytes};
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Header, RlpEncodable};
use sha3::{Digest, Keccak256};

use crate::types::hash::H160;

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
    fn decode(buf: &mut &[u8], hash: &str) -> Result<Self, DecodeError> {
        let tx_header = match Header::decode(buf) {
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
        let h = match Header::decode(payload_view) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode gas price header: {:?}", e);
                return Err(e);
            }
        };

        payload_view.advance(h.payload_length);

        let recipient = match H160::decode(payload_view) {
            Ok(n) => n,
            Err(e) => {
                println!(
                    "Failed to decode recipient: {:?}, for hash tx: https://bscscan.com/tx/0x{}",
                    e, hash
                );
                return Err(e);
            }
        };

        // skip value
        let h = match Header::decode(payload_view) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode value header: {:?}", e);
                return Err(e);
            }
        };

        payload_view.advance(h.payload_length);

        let _data = match Bytes::decode(payload_view) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode data: {:?}", e);
                return Err(e);
            }
        };

        println!(
            "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, hash
        );

        // we skip v, r, s
        buf.advance(tx_header.payload_length);

        Ok(Transaction::default())
    }
}

pub fn decode_txs(buf: &mut &[u8]) -> Result<Vec<Transaction>, DecodeError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeError::UnexpectedString);
    }

    let payload_view = &mut &buf[..h.payload_length];
    while !payload_view.is_empty() {
        let hash = eth_tx_hash(payload_view);
        Transaction::decode(payload_view, &hash)?;
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
