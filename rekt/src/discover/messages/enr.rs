use open_fastrlp::RlpEncodable;

use crate::types::hash::H256;

#[derive(Debug, Clone, RlpEncodable)]
pub struct EnrResponseMessage {
    pub request_hash: H256,
    pub enr: enr::Enr<secp256k1::SecretKey>,
}

impl EnrResponseMessage {
    pub fn new(request_hash: H256, enr: enr::Enr<secp256k1::SecretKey>) -> Self {
        Self { request_hash, enr }
    }
}
