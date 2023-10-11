use bytes::{Buf, Bytes};
use derive_more::Display;
use ethers::types::{U128, U256};
use open_fastrlp::{Decodable, DecodeError, Header, HeaderInfo, RlpEncodable};
use sha3::{Digest, Keccak256};
use thiserror::Error;

use crate::{
    enemies::enemy::Enemy,
    token::tokens_to_buy::{get_token, mark_token_as_bought, tx_is_enable_buy},
    types::hash::H160,
};

#[derive(Debug, Clone, PartialEq, Eq, Display)]
enum TxType {
    Legacy,
    AccessList,
    DynamicFee,
    Blob,
}

impl TryFrom<u8> for TxType {
    type Error = DecodeTxError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(TxType::Legacy),
            0x01 => Ok(TxType::AccessList),
            0x02 => Ok(TxType::DynamicFee),
            0x03 => Ok(TxType::Blob),
            _ => Err(DecodeTxError::UnknownTxType),
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
    fn decode(buf: &mut &[u8]) -> Result<usize, DecodeTxError> {
        let tx_header_info = HeaderInfo::decode(buf)?;

        let rlp_decoding_is_of_legacy_tx = tx_header_info.list;
        if rlp_decoding_is_of_legacy_tx {
            let hash = eth_tx_hash(TxType::Legacy, &buf[..tx_header_info.total_len]);
            let tx_metadata = Header::decode_from_info(buf, tx_header_info)?;

            return Transaction::decode_legacy(buf, tx_metadata, hash);
        }

        let _parse_rlp_header_and_advance_buf = Header::decode_from_info(buf, tx_header_info)?;
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
            _ => Err(DecodeTxError::UnknownTxType),
        }
    }

    fn decode_legacy(
        buf: &mut &[u8],
        rlp_header: Header,
        hash: String,
    ) -> Result<usize, DecodeTxError> {
        let payload_view = &mut &buf[..rlp_header.payload_length];

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;

        let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

        let recipient = match ethers::types::H160::decode(payload_view) {
            Ok(v) => v,
            Err(_errored_because_this_tx_is_contract_creation) => {
                return Ok(rlp_header.payload_length);
            }
        };

        let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view)?;
        let data = Bytes::decode(payload_view)?;

        if let Some((bot, token)) = Enemy::enemy_is_preparing_to_buy_token(&data) {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
            println!(
            "[{now}] OLD TX BOT {bot} PREPARED: nonce: {}, gas_price: {}, to: {} \n token: https://bscscan.com/token/{:#x}, tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, token, hash
        );

            return Ok(rlp_header.payload_length);
        }

        let token = match get_token(&recipient) {
            None => return Ok(rlp_header.payload_length),
            Some(t) => t,
        };

        if !tx_is_enable_buy(token, &data) {
            return Ok(rlp_header.payload_length);
        }

        mark_token_as_bought(token.buy_token_address);

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
        println!(
            "[{now}] OLD TX: nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, hash
        );

        //  we skip v, r, s
        Ok(rlp_header.payload_length)
    }

    fn decode_dynamic_and_blob_tx_types(
        tx_type: TxType,
        buf: &mut &[u8],
    ) -> Result<usize, DecodeTxError> {
        let tx_header_info = HeaderInfo::decode(buf)?;
        let hash = eth_tx_hash(tx_type, &buf[..tx_header_info.total_len]);
        let rlp_header = Header::decode_from_info(buf, tx_header_info)?;

        if !rlp_header.list {
            return Err(DecodeTxError::from(DecodeError::UnexpectedString));
        }
        let payload_view = &mut &buf[..rlp_header.payload_length];

        let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;
        let max_price_per_gas = u64::decode(payload_view)?;

        let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

        let recipient = match ethers::types::H160::decode(payload_view) {
            Ok(v) => v,
            Err(_errored_because_this_tx_is_contract_creation) => {
                return Ok(rlp_header.payload_length);
            }
        };

        let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
        let data = Bytes::decode(payload_view)?;

        if let Some((bot, token)) = Enemy::enemy_is_preparing_to_buy_token(&data) {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
            println!(
            "[{now}] NEW TX BOT {bot} PREPARED: nonce: {}, gas_price: {}, to: {}, \n  token: https://bscscan.com/token/{:#x}, tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, token, hash
        );

            return Ok(rlp_header.payload_length);
        }

        let token = match get_token(&recipient) {
            None => return Ok(rlp_header.payload_length),
            Some(t) => t,
        };

        if !tx_is_enable_buy(token, &data) {
            return Ok(rlp_header.payload_length);
        }

        mark_token_as_bought(token.buy_token_address);
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
        println!(
            "[{now}] NEW TX: nonce: {}, gas_price: {},max gas per price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, max_price_per_gas,  recipient, hash
        );

        Ok(rlp_header.payload_length)
    }

    fn decode_access_list_tx_type(
        tx_type: TxType,
        buf: &mut &[u8],
    ) -> Result<usize, DecodeTxError> {
        let tx_header_info = HeaderInfo::decode(buf)?;
        let hash = eth_tx_hash(tx_type, &buf[..tx_header_info.total_len]);
        let rlp_header = Header::decode_from_info(buf, tx_header_info)?;

        if !rlp_header.list {
            return Err(DecodeTxError::from(DecodeError::UnexpectedString));
        }
        let payload_view = &mut &buf[..rlp_header.payload_length];

        let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

        let nonce = u64::decode(payload_view)?;
        let gas_price = u64::decode(payload_view)?;

        let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

        let recipient = match ethers::types::H160::decode(payload_view) {
            Ok(v) => v,
            Err(_errored_because_this_tx_is_contract_creation) => {
                return Ok(rlp_header.payload_length);
            }
        };

        let token = match get_token(&recipient) {
            None => return Ok(rlp_header.payload_length),
            Some(t) => t,
        };

        let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
        let data = Bytes::decode(payload_view)?;

        if !tx_is_enable_buy(token, &data) {
            return Ok(rlp_header.payload_length);
        }

        mark_token_as_bought(token.buy_token_address);

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
        println!(
            "[{now}] ACCESS TX: nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/0x{}",
            nonce, gas_price, recipient, hash
        );
        //  we skip v, r, s
        Ok(rlp_header.payload_length)
    }
}

pub fn decode_txs_request(buf: &mut &[u8]) -> Result<(), DecodeTxError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }
    // skip decoding request id
    HeaderInfo::skip_next_item(buf)?;
    decode_txs(buf)
}

pub fn decode_txs(buf: &mut &[u8]) -> Result<(), DecodeTxError> {
    let metadata = Header::decode(buf)?;
    if !metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
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
                return Err(e);
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

#[derive(Error, Debug)]
pub enum DecodeTxError {
    #[error("Decode error: {0}")]
    MessageDecodeError(#[from] open_fastrlp::DecodeError),
    #[error("TX is contract creation")]
    ContractCreation,
    #[error("TX is of unknown type")]
    UnknownTxType,
}
