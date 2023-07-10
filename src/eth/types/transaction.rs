use bytes::Bytes;
use ethers::types::{U128, U256};
use open_fastrlp::{RlpDecodable, RlpEncodable};

use crate::types::hash::H160;

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct Transaction {
    pub nonce: U256,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub to: H160,
    pub value: U256,
    pub data: Bytes,
}
