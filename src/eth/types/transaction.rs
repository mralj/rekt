use bytes::{Buf, Bytes};
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Header, RlpEncodable};
use sha3::{Digest, Keccak256};

use crate::types::hash::H160;

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable)]
pub struct Transaction {
    pub nonce: U256,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub to: H160,
    pub value: U256,
    pub data: Bytes,
}

//Nonce
// Gas Price
// Gas Limit
// Recipient Address
// Value
// Data
// v
// r
// s

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
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode header: {:?}", e);
                return Err(e);
            }
        };

        if !h.list {
            return Err(DecodeError::UnexpectedString);
        }

        let buf = &mut &buf[..h.payload_length];
        let nonce = match u64::decode(buf) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode nonce: {:?}", e);
                return Err(e);
            }
        };

        let gas_price = match u64::decode(buf) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode gas price: {:?}", e);
                return Err(e);
            }
        };

        // skip gas limit
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode gas price header: {:?}", e);
                return Err(e);
            }
        };

        buf.advance(h.payload_length);

        let recipient = match H160::decode(buf) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode recipient: {:?}", e);
                return Err(e);
            }
        };

        // skip value
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode value header: {:?}", e);
                return Err(e);
            }
        };

        buf.advance(h.payload_length);

        let _data = match Bytes::decode(buf) {
            Ok(n) => n,
            Err(e) => {
                println!("Failed to decode data: {:?}", e);
                return Err(e);
            }
        };

        //skip, r,s,v
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode v header: {:?}", e);
                return Err(e);
            }
        };

        buf.advance(h.payload_length);
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode r header: {:?}", e);
                return Err(e);
            }
        };

        buf.advance(h.payload_length);
        let h = match Header::decode(buf) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to decode s header: {:?}", e);
                return Err(e);
            }
        };
        buf.advance(h.payload_length);

        tracing::info!(
            "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce,
            gas_price,
            recipient,
            hash
        );

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
