use bytes::{Buf, Bytes};
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Header, HeaderInfo, RlpEncodable};
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
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, Default)]
pub struct Transaction {
    pub nonce: U256,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub to: H160,
    pub value: U256,
    pub data: Bytes,
}

impl Transaction {
    fn decode(buf: &mut &[u8]) -> Result<usize, DecodeError> {
        let tx_header_info = HeaderInfo::decode(buf)?;
        let hash = eth_tx_hash(&buf[..tx_header_info.total_len]);

        let tx_metadata = Header::decode_from_info(buf, tx_header_info)?;
        if !tx_metadata.list {
            return Err(DecodeError::UnexpectedString);
        }

        let payload_view = &mut &buf[..tx_metadata.payload_length];

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;

        // skip gas limit
        HeaderInfo::skip_next_item(payload_view)?;

        let recipient = H160::decode(payload_view)?;

        // skip value
        HeaderInfo::skip_next_item(payload_view)?;

        let data = Bytes::decode(payload_view)?;

        println!(
            "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, hash
        );

        //  we skip v, r, s
        Ok(tx_metadata.payload_length)
    }
}

pub fn decode_txs(buf: &mut &[u8]) -> Result<(), DecodeError> {
    let metadata = Header::decode(buf)?;
    if !metadata.list {
        return Err(DecodeError::UnexpectedString);
    }

    // note that for processing we are just "viewing" into the data
    // original buffer remains the same
    // the data for processing is of length specified in the RLP header a.k.a metadata
    let payload_view = &mut &buf[..metadata.payload_length];
    while !payload_view.is_empty() {
        let tx_byte_size = Transaction::decode(payload_view)?;
        payload_view.advance(tx_byte_size);
    }

    Ok(())
}

fn eth_tx_hash(raw_tx: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(raw_tx);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}
