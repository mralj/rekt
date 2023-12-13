use std::str::FromStr;

use bytes::{Buf, Bytes};
use chrono::{DateTime, Utc};
use ethers::types::Address;
use open_fastrlp::{Decodable, DecodeError, Header, HeaderInfo};
use sha3::{Digest, Keccak256};
use static_init::dynamic;

use super::{cache, errors::DecodeTxError, types::TxType};
use crate::{
    constants::{
        TOKEN_IN_TX_ENDS_AT, TOKEN_IN_TX_ENDS_AT_POSSIBLE_POSITION_2, TOKEN_IN_TX_STARTS_AT,
        TOKEN_IN_TX_STARTS_AT_POSSIBLE_POSITION_2,
    },
    p2p::peer::BUY_IS_IN_PROGRESS,
    token::{
        token::Token,
        tokens_to_buy::{
            get_token_to_buy, tx_is_enable_buy, tx_nonce_is_ok, PCS_LIQ, TOKENS_TO_BUY,
        },
    },
    types::hash::H256,
};

#[dynamic]
pub static PCS_V2_ROUTER: Address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E")
    .expect("Invalid PCS V2 router address");

#[dynamic]
pub static PCS_V3_LIQ_CONTRACT: Address =
    Address::from_str("0x46A15B0b27311cedF172AB29E4f4766fbE7F4364")
        .expect("Invalid PCS V2 router address");

pub enum TxDecodingResult {
    NoBuy(usize),
    Buy(BuyTokenInfo),
}

pub struct BuyTokenInfo {
    pub token: Token,
    pub gas_price: u64,
    pub hash: H256,
    pub time: DateTime<Utc>,
    pub was_tx_direct: bool,
}

impl BuyTokenInfo {
    pub fn new(token: Token, gas_price: u64, hash: H256) -> Self {
        Self {
            token,
            gas_price,
            hash,
            time: chrono::Utc::now(),
            was_tx_direct: false,
        }
    }

    pub fn set_tx_direct(&mut self, tx_direct: bool) {
        self.was_tx_direct = tx_direct
    }
}

pub fn decode_txs_request(buf: &mut &[u8]) -> Result<Option<BuyTokenInfo>, DecodeTxError> {
    let h = Header::decode(buf)?;
    if !h.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }

    let _skip_decoding_request_id = HeaderInfo::skip_next_item(buf)?;
    decode_txs(buf, false)
}

pub fn decode_txs(buf: &mut &[u8], direct: bool) -> Result<Option<BuyTokenInfo>, DecodeTxError> {
    let metadata = Header::decode(buf)?;
    if !metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }

    // note that for processing we are just "viewing" into the data
    // original buffer remains the same
    // the data for processing is of length specified in the RLP header a.k.a metadata
    let payload_view = &mut &buf[..metadata.payload_length];
    while !payload_view.is_empty() {
        match decode_tx(payload_view)? {
            TxDecodingResult::Buy(mut buy_info) => {
                buy_info.set_tx_direct(direct);
                return Ok(Some(buy_info));
            }
            TxDecodingResult::NoBuy(tx_size) => {
                payload_view.advance(tx_size);
                continue;
            }
        };
    }

    Ok(None)
}

fn decode_tx(buf: &mut &[u8]) -> Result<TxDecodingResult, DecodeTxError> {
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
        TxType::DynamicFee | TxType::Blob => {
            buf.advance(1);
            decode_dynamic_and_blob_tx_types(tx_type_flag, buf)
        }
        TxType::AccessList => {
            buf.advance(1);
            decode_access_list_tx_type(tx_type_flag, buf)
        }
        TxType::Legacy => unreachable!(),
    }
}

fn decode_legacy(
    buf: &mut &[u8],
    tx_metadata: HeaderInfo,
) -> Result<TxDecodingResult, DecodeTxError> {
    let hash = eth_tx_hash(TxType::Legacy, &buf[..tx_metadata.total_len]);
    if cache::mark_as_fetched(&hash) == cache::TxCacheStatus::Fetched {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.total_len));
    }

    let tx_metadata = Header::decode_from_info(buf, tx_metadata)?;

    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let nonce = u64::decode(payload_view)?;
    if !tx_nonce_is_ok(nonce) {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
    }

    let gas_price = u64::decode(payload_view)?;
    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
        }
    };

    if unsafe { PCS_LIQ } && recipient_is_to_pcs(&recipient) {
        return handle_pcs(tx_metadata, payload_view, hash, recipient, gas_price);
    }
    handle_token(tx_metadata, payload_view, hash, nonce, gas_price, recipient)
}

fn decode_dynamic_and_blob_tx_types(
    tx_type: TxType,
    buf: &mut &[u8],
) -> Result<TxDecodingResult, DecodeTxError> {
    let tx_metadata = HeaderInfo::decode(buf)?;
    let hash = eth_tx_hash(tx_type, &buf[..tx_metadata.total_len]);
    if cache::mark_as_fetched(&hash) == cache::TxCacheStatus::Fetched {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.total_len));
    }

    let tx_metadata = Header::decode_from_info(buf, tx_metadata)?;
    if !tx_metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }
    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

    let nonce = u64::decode(payload_view)?;
    if !tx_nonce_is_ok(nonce) {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
    }

    let gas_price = u64::decode(payload_view)?;

    let _skip_max_price_per_gas = HeaderInfo::skip_next_item(payload_view)?;
    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
        }
    };

    if unsafe { PCS_LIQ } && recipient_is_to_pcs(&recipient) {
        return handle_pcs(tx_metadata, payload_view, hash, recipient, gas_price);
    }
    handle_token(tx_metadata, payload_view, hash, nonce, gas_price, recipient)
}

fn decode_access_list_tx_type(
    tx_type: TxType,
    buf: &mut &[u8],
) -> Result<TxDecodingResult, DecodeTxError> {
    let tx_metadata = HeaderInfo::decode(buf)?;
    let hash = eth_tx_hash(tx_type, &buf[..tx_metadata.total_len]);

    if cache::mark_as_fetched(&hash) == cache::TxCacheStatus::Fetched {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.total_len));
    }

    let tx_metadata = Header::decode_from_info(buf, tx_metadata)?;

    if !tx_metadata.list {
        return Err(DecodeTxError::from(DecodeError::UnexpectedString));
    }
    let payload_view = &mut &buf[..tx_metadata.payload_length];

    let _skip_decoding_chain_id = HeaderInfo::skip_next_item(payload_view)?;

    let nonce = u64::decode(payload_view)?;
    if !tx_nonce_is_ok(nonce) {
        return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
    }

    let gas_price = u64::decode(payload_view)?;
    let _skip_decoding_gas_limit = HeaderInfo::skip_next_item(payload_view)?;

    let recipient = match ethers::types::H160::decode(payload_view) {
        Ok(v) => v,
        Err(_errored_because_this_tx_is_contract_creation) => {
            return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
        }
    };

    if unsafe { PCS_LIQ } && recipient_is_to_pcs(&recipient) {
        return handle_pcs(tx_metadata, payload_view, hash, recipient, gas_price);
    }
    handle_token(tx_metadata, payload_view, hash, nonce, gas_price, recipient)
}

fn eth_tx_hash(tx_type: TxType, raw_tx: &[u8]) -> H256 {
    let mut hasher = Keccak256::new();
    if tx_type != TxType::Legacy {
        hasher.update(&[tx_type as u8]);
    }
    hasher.update(raw_tx);
    H256::from_slice(&hasher.finalize())
}

fn handle_token(
    tx_metadata: Header,
    payload_view: &mut &[u8],
    hash: H256,
    nonce: u64,
    gas_price: u64,
    recipient: ethers::types::H160,
) -> Result<TxDecodingResult, DecodeTxError> {
    let (token, index) = match get_token_to_buy(&recipient, nonce) {
        Some((t, i)) => (t, i),
        None => return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length)),
    };

    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
    let data = Bytes::decode(payload_view)?;

    let token = match tx_is_enable_buy(token, index, &data) {
        Some(token) => token,
        None => return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length)),
    };

    unsafe {
        BUY_IS_IN_PROGRESS = true;
    }

    Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
        token, gas_price, hash,
    )))
}

fn handle_pcs(
    tx_metadata: Header,
    payload_view: &mut &[u8],
    hash: H256,
    recipient: ethers::types::H160,
    gas_price: u64,
) -> Result<TxDecodingResult, DecodeTxError> {
    if recipient == *PCS_V2_ROUTER {
        return handle_pcs_v2(tx_metadata, payload_view, hash, gas_price);
    }

    return handle_pcs_v3(tx_metadata, payload_view, hash, gas_price);
}

fn handle_pcs_v2(
    tx_metadata: Header,
    payload_view: &mut &[u8],
    hash: H256,
    gas_price: u64,
) -> Result<TxDecodingResult, DecodeTxError> {
    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
    let data = Bytes::decode(payload_view)?;

    if data.starts_with(&[0xe8, 0xe3, 0x37, 0x00]) {
        let first_position_token = &data[TOKEN_IN_TX_STARTS_AT..TOKEN_IN_TX_ENDS_AT];
        if let Some((token, index)) =
            get_token_to_buy(&Address::from_slice(first_position_token), 0)
        {
            if !token.liq_will_be_added_via_pcs {
                return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
            }
            unsafe {
                BUY_IS_IN_PROGRESS = true;
            }

            let token = unsafe { TOKENS_TO_BUY.swap_remove(index) };

            return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                token, gas_price, hash,
            )));
        }

        let second_position_token = &data
            [TOKEN_IN_TX_STARTS_AT_POSSIBLE_POSITION_2..TOKEN_IN_TX_ENDS_AT_POSSIBLE_POSITION_2];

        if let Some((token, index)) =
            get_token_to_buy(&Address::from_slice(second_position_token), 0)
        {
            if !token.liq_will_be_added_via_pcs {
                return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
            }
            unsafe {
                BUY_IS_IN_PROGRESS = true;
            }

            let token = unsafe { TOKENS_TO_BUY.swap_remove(index) };

            return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                token, gas_price, hash,
            )));
        }
    }

    if data.starts_with(&[0xf3, 0x05, 0xd7, 0x19]) {
        let first_position_token = &data[TOKEN_IN_TX_STARTS_AT..TOKEN_IN_TX_ENDS_AT];
        if let Some((token, index)) =
            get_token_to_buy(&Address::from_slice(first_position_token), 0)
        {
            if !token.liq_will_be_added_via_pcs {
                return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
            }
            unsafe {
                BUY_IS_IN_PROGRESS = true;
            }

            let token = unsafe { TOKENS_TO_BUY.swap_remove(index) };

            return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                token, gas_price, hash,
            )));
        }
    }

    Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length))
}

fn handle_pcs_v3(
    tx_metadata: Header,
    payload_view: &mut &[u8],
    hash: H256,
    gas_price: u64,
) -> Result<TxDecodingResult, DecodeTxError> {
    let _skip_decoding_value = HeaderInfo::skip_next_item(payload_view);
    let data = Bytes::decode(payload_view)?;

    if data.starts_with(&[0x88, 0x31, 0x64, 0x56]) {
        let first_position_token = &data[TOKEN_IN_TX_STARTS_AT..TOKEN_IN_TX_ENDS_AT];
        if let Some((token, index)) =
            get_token_to_buy(&Address::from_slice(first_position_token), 0)
        {
            if !token.liq_will_be_added_via_pcs {
                return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
            }
            unsafe {
                BUY_IS_IN_PROGRESS = true;
            }

            let token = unsafe { TOKENS_TO_BUY.swap_remove(index) };

            return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                token, gas_price, hash,
            )));
        }

        let second_position_token = &data
            [TOKEN_IN_TX_STARTS_AT_POSSIBLE_POSITION_2..TOKEN_IN_TX_ENDS_AT_POSSIBLE_POSITION_2];

        if let Some((token, index)) =
            get_token_to_buy(&Address::from_slice(second_position_token), 0)
        {
            if !token.liq_will_be_added_via_pcs {
                return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
            }
            unsafe {
                BUY_IS_IN_PROGRESS = true;
            }

            let token = unsafe { TOKENS_TO_BUY.swap_remove(index) };

            return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                token, gas_price, hash,
            )));
        }
    }

    if data.starts_with(&[0xac, 0x96, 0x50, 0xd8]) {
        if !contains(&data, &[0x13, 0xea, 0xd5, 0x62]) {
            return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
        }

        if !contains(&data, &[0x88, 0x31, 0x64, 0x56]) {
            return Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length));
        }

        unsafe {
            for (i, t) in TOKENS_TO_BUY.iter().enumerate() {
                if !t.liq_will_be_added_via_pcs {
                    continue;
                }

                if contains(&data, t.buy_token_address.as_bytes()) {
                    BUY_IS_IN_PROGRESS = true;
                    let token = TOKENS_TO_BUY.swap_remove(i);
                    return Ok(TxDecodingResult::Buy(BuyTokenInfo::new(
                        token, gas_price, hash,
                    )));
                }
            }
        }
    }

    Ok(TxDecodingResult::NoBuy(tx_metadata.payload_length))
}

fn recipient_is_to_pcs(recipient: &ethers::types::H160) -> bool {
    *recipient == *PCS_V2_ROUTER || *recipient == *PCS_V3_LIQ_CONTRACT
}

fn contains(bytes: &Bytes, slice: &[u8]) -> bool {
    bytes.windows(slice.len()).any(|window| window == slice)
}
