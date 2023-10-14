use std::time::Instant;

use bytes::{Buf, Bytes};
use open_fastrlp::{Decodable, DecodeError, Header, HeaderInfo};
use sha3::{Digest, Keccak256};

use crate::{
    enemies::enemy::Enemy,
    eth::transactions::cache,
    token::tokens_to_buy::{get_token, mark_token_as_bought, tx_is_enable_buy},
    types::hash::H256,
};

use super::{errors::DecodeTxError, types::TxType};

pub fn decode_txs_request(buf: &mut &[u8]) -> Result<(), DecodeTxError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }

    let _skip_decoding_request_id = HeaderInfo::skip_next_item(buf)?;
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
        //TODO: short-circuit this once we are 100% sure the code works
        let tx_size = match decode_tx(payload_view) {
            Ok(v) => v,
            Err(e) => {
                println!("Could not decode tx: {:?}", e);
                return Err(e);
            }
        };
        payload_view.advance(tx_size);
    }

    Ok(())
}

fn decode_tx(buf: &mut &[u8]) -> Result<usize, DecodeTxError> {
    let tx_metadata = HeaderInfo::decode(buf)?;

    //NOTE:
    //This is from the docs (https://eips.ethereum.org/EIPS/eip-2718)
    //Clients can differentiate between the legacy transactions and typed transactions by looking at the first byte.
    //If it starts with a value in the range [0, 0x7f] then it is a new transaction type,
    //if it starts with a value in the range [0xc0, 0xfe] then it is a legacy transaction type.
    //0xff is not realistic for an RLP encoded transaction, so it is reserved for future use as an extension sentinel value.

    let rlp_decoding_is_of_legacy_tx = tx_metadata.list;
    if rlp_decoding_is_of_legacy_tx {
        return decode_legacy(buf, tx_metadata);
    }

    let _typed_tx_metadata = Header::decode_from_info(buf, tx_metadata)?;
    let tx_type_flag = TxType::try_from(buf[0])?;
    match tx_type_flag {
        TxType::AccessList => {
            buf.advance(1);
            decode_access_list_tx_type(tx_type_flag, buf)
        }
        TxType::DynamicFee | TxType::Blob => {
            buf.advance(1);
            decode_dynamic_and_blob_tx_types(tx_type_flag, buf)
        }
        _ => Err(DecodeTxError::UnknownTxType),
    }
}

fn decode_legacy(buf: &mut &[u8], tx_header_metadata: HeaderInfo) -> Result<usize, DecodeTxError> {
    let hash = eth_tx_hash(TxType::Legacy, &buf[..tx_header_metadata.total_len]);
    let start = Instant::now();
    if cache::insert(&hash) {
        return Ok(tx_header_metadata.total_len);
    }
    println!("Inserting tx into cache took: {:?}", start.elapsed());
    let tx_metadata = Header::decode_from_info(buf, tx_header_metadata)?;
    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let nonce = u64::decode(payload_view)?;
    let gas_price = u64::decode(payload_view)?;

    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(tx_metadata.payload_length);
        }
    };

    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view)?;
    let data = Bytes::decode(payload_view)?;

    if let Some((bot, token)) = Enemy::enemy_is_preparing_to_buy_token(&data) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
        println!(
            "[{now}] OLD TX BOT {bot} PREPARED: nonce: {}, gas_price: {}, to: {} \n token: https://bscscan.com/token/{:#x}, tx: https://bscscan.com/tx/{:#x}",
            nonce, gas_price, recipient, token, hash
        );

        return Ok(tx_metadata.payload_length);
    }

    let token = match get_token(&recipient) {
        None => return Ok(tx_metadata.payload_length),
        Some(t) => t,
    };

    if !tx_is_enable_buy(token, &data) {
        return Ok(tx_metadata.payload_length);
    }

    mark_token_as_bought(token.buy_token_address);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
    println!(
        "[{now}] OLD TX: nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/{:#x}",
        nonce, gas_price, recipient, hash
    );

    Ok(tx_metadata.payload_length)
}

fn decode_dynamic_and_blob_tx_types(
    tx_type: TxType,
    buf: &mut &[u8],
) -> Result<usize, DecodeTxError> {
    let tx_header_metadata = HeaderInfo::decode(buf)?;

    let hash = eth_tx_hash(tx_type, &buf[..tx_header_metadata.total_len]);
    let start = Instant::now();
    if cache::insert(&hash) {
        return Ok(tx_header_metadata.total_len);
    }

    println!("Inserting tx into cache took: {:?}", start.elapsed());
    let tx_metadata = Header::decode_from_info(buf, tx_header_metadata)?;

    if !tx_metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }
    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

    let nonce = u64::decode(payload_view)?;
    let gas_price = u64::decode(payload_view)?;
    let max_price_per_gas = u64::decode(payload_view)?;

    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(tx_metadata.payload_length);
        }
    };

    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
    let data = Bytes::decode(payload_view)?;

    if let Some((bot, token)) = Enemy::enemy_is_preparing_to_buy_token(&data) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
        println!(
            "[{now}] NEW TX BOT {bot} PREPARED: nonce: {}, gas_price: {}, to: {}, \n  token: https://bscscan.com/token/{:#x}, tx: https://bscscan.com/tx/{:#x}",
            nonce, gas_price, recipient, token, hash
        );

        return Ok(tx_metadata.payload_length);
    }

    let token = match get_token(&recipient) {
        None => return Ok(tx_metadata.payload_length),
        Some(t) => t,
    };

    if !tx_is_enable_buy(token, &data) {
        return Ok(tx_metadata.payload_length);
    }

    mark_token_as_bought(token.buy_token_address);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
    println!(
            "[{now}] NEW TX: nonce: {}, gas_price: {},max gas per price: {}, to: {},  tx: https://bscscan.com/tx/{:#x}",
            nonce, gas_price, max_price_per_gas,  recipient, hash
        );

    Ok(tx_metadata.payload_length)
}

fn decode_access_list_tx_type(tx_type: TxType, buf: &mut &[u8]) -> Result<usize, DecodeTxError> {
    let tx_header_metadata = HeaderInfo::decode(buf)?;

    let hash = eth_tx_hash(tx_type, &buf[..tx_header_metadata.total_len]);
    if cache::insert(&hash) {
        return Ok(tx_header_metadata.total_len);
    }

    let tx_metadata = Header::decode_from_info(buf, tx_header_metadata)?;

    if !tx_metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }
    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

    let nonce = u64::decode(payload_view)?;
    let gas_price = u64::decode(payload_view)?;

    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(tx_metadata.payload_length);
        }
    };

    let token = match get_token(&recipient) {
        None => return Ok(tx_metadata.payload_length),
        Some(t) => t,
    };

    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
    let data = Bytes::decode(payload_view)?;

    if !tx_is_enable_buy(token, &data) {
        return Ok(tx_metadata.payload_length);
    }

    mark_token_as_bought(token.buy_token_address);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.6f");
    println!(
        "[{now}] ACCESS TX: nonce: {}, gas_price: {}, to: {},  tx: https://bscscan.com/tx/{:#x}",
        nonce, gas_price, recipient, hash
    );

    Ok(tx_metadata.payload_length)
}

fn eth_tx_hash(tx_type: TxType, raw_tx: &[u8]) -> H256 {
    let mut hasher = Keccak256::new();
    if tx_type != TxType::Legacy {
        hasher.update(&[tx_type as u8]);
    }
    hasher.update(raw_tx);
    H256::from_slice(&hasher.finalize())
}
