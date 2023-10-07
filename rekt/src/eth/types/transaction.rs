use bytes::{Buf, Bytes};
use derive_more::Display;
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Header, HeaderInfo, RlpEncodable};
use sha3::{Digest, Keccak256};

use crate::types::hash::H160;

#[derive(Debug, Clone, PartialEq, Eq, Display)]
enum TxType {
    Legacy,
    AccessList,
    DynamicFee,
    Blob,
}

impl TryFrom<u8> for TxType {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(TxType::AccessList),
            0x02 => Ok(TxType::DynamicFee),
            0x03 => Ok(TxType::Blob),
            _ => Err(DecodeError::UnexpectedString),
        }
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
        let tx_header_info = match HeaderInfo::decode(buf) {
            Ok(v) => v,
            Err(e) => {
                println!("Decode general: Could not decode header info: {:?}", e);
                return Err(e);
            }
        };

        let rlp_decoding_is_of_legacy_tx = tx_header_info.list;
        if rlp_decoding_is_of_legacy_tx {
            let hash = eth_tx_hash(TxType::Legacy, &buf[..tx_header_info.total_len]);
            let tx_metadata = match Header::decode_from_info(buf, tx_header_info) {
                Ok(v) => v,
                Err(e) => {
                    println!("Decode legacy : Could not decode metadata: {:?}", e);
                    return Err(e);
                }
            };

            return Transaction::decode_legacy(buf, tx_metadata, hash);
        }

        let _parse_rlp_header_and_advance_buf = match Header::decode_from_info(buf, tx_header_info)
        {
            Ok(v) => v,
            Err(e) => {
                println!("Decode typed: Could not decode metadata: {:?}", e);
                return Err(e);
            }
        };
        let tx_type_flag = TxType::try_from(buf[0])?;
        match tx_type_flag {
            TxType::AccessList => {
                buf.advance(1);
                Transaction::decode_access_list_tx_type(tx_type_flag, buf)
            }
            TxType::DynamicFee | TxType::Blob => {
                buf.advance(1);
                Transaction::decode_dynamic_and_blob_tx_types(tx_type_flag, buf)
            }
            _ => Err(DecodeError::UnexpectedString),
        }
    }

    fn decode_legacy(
        buf: &mut &[u8],
        rlp_header: Header,
        hash: String,
    ) -> Result<usize, DecodeError> {
        let payload_view = &mut &buf[..rlp_header.payload_length];

        let nonce = match u64::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Decode legacy: Could not decode nonce: {:?}", e);
                return Err(e);
            }
        };
        let gas_price = match u64::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Decode legacy: Could not decode gas price: {:?}", e);
                return Err(e);
            }
        };
        //
        // skip gas limit
        match HeaderInfo::skip_next_item(payload_view) {
            Ok(_) => {}
            Err(e) => {
                println!("Decode legacy: Could not skip gas limit: {:?}", e);
                return Err(e);
            }
        };

        let recipient = match H160::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Decode legacy: Could not decode recipient: {:?}", e);
                return Err(e);
            }
        };
        // skip value
        match HeaderInfo::skip_next_item(payload_view) {
            Ok(_) => {}
            Err(e) => {
                println!("Decode legacy: Could not skip value: {:?}", e);
                return Err(e);
            }
        }
        let data = match Bytes::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Decode legacy: Could not decode data: {:?}", e);
                return Err(e);
            }
        };

        //print!("nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
        //     nonce, gas_price, recipient, hash
        // );
        //
        //  we skip v, r, s
        Ok(rlp_header.payload_length)
    }

    fn decode_dynamic_and_blob_tx_types(
        tx_type: TxType,
        buf: &mut &[u8],
    ) -> Result<usize, DecodeError> {
        let tx_header_info = match HeaderInfo::decode(buf) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode header info: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };
        let hash = eth_tx_hash(tx_type, &buf[..tx_header_info.total_len]);
        let rlp_header = match Header::decode_from_info(buf, tx_header_info) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode header: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };

        if !rlp_header.list {
            println!("Expected list");
            return Err(DecodeError::UnexpectedString);
        }
        let payload_view = &mut &buf[..rlp_header.payload_length];

        // skip chain id
        HeaderInfo::skip_next_item(payload_view)?;

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;
        let max_price_per_gas = u64::decode(payload_view)?;

        // skip gas limit
        HeaderInfo::skip_next_item(payload_view)?;

        let recipient = match H160::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Typed Could not decode recipient: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };

        // skip value
        HeaderInfo::skip_next_item(payload_view)?;

        let data = Bytes::decode(payload_view)?;

        // println!(
        //     "nonce: {}, gas_price: {},max gas per price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
        //     nonce, gas_price, max_price_per_gas,  recipient, hash
        // );

        //  we skip v, r, s
        Ok(rlp_header.payload_length)
    }

    fn decode_access_list_tx_type(tx_type: TxType, buf: &mut &[u8]) -> Result<usize, DecodeError> {
        let tx_header_info = match HeaderInfo::decode(buf) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode header info: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };
        let hash = eth_tx_hash(tx_type, &buf[..tx_header_info.total_len]);
        let rlp_header = match Header::decode_from_info(buf, tx_header_info) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode header: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };

        if !rlp_header.list {
            println!("Expected list");
            return Err(DecodeError::UnexpectedString);
        }
        let payload_view = &mut &buf[..rlp_header.payload_length];

        // skip chain id
        HeaderInfo::skip_next_item(payload_view)?;

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;

        // skip gas limit
        HeaderInfo::skip_next_item(payload_view)?;
        let recipient = H160::decode(payload_view)?;
        // skip value
        HeaderInfo::skip_next_item(payload_view)?;
        let data = Bytes::decode(payload_view)?;

        // println!(
        //     "nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
        //     nonce, gas_price, recipient, hash
        // );
        Ok(rlp_header.payload_length)
    }
}

pub fn decode_txs_request(buf: &mut &[u8]) -> Result<(), DecodeError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeError::UnexpectedString);
    }
    // skip decoding request id
    HeaderInfo::skip_next_item(buf)?;
    decode_txs(buf)
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
        let tx_byte_size = match Transaction::decode(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode tx: {:?}", e);
                return Err(DecodeError::UnexpectedString);
            }
        };
        payload_view.advance(tx_byte_size);
    }

    Ok(())
}

fn eth_tx_hash(tx_type: TxType, raw_tx: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    if tx_type != TxType::Legacy {
        hasher.update(&[tx_type as u8]);
    }
    hasher.update(raw_tx);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}
