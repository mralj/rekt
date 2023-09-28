use std::time::{SystemTime, UNIX_EPOCH};

use open_fastrlp::{Decodable, RlpDecodable, RlpEncodable};

use crate::{blockchain::fork::ForkId, types::hash::H256};

use super::discover_message::DEFAULT_MESSAGE_EXPIRATION;

#[derive(Debug, Clone, RlpEncodable)]
pub struct EnrRequest {
    pub expiration: u64,
}

impl EnrRequest {
    pub fn new() -> Self {
        Self {
            expiration: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + DEFAULT_MESSAGE_EXPIRATION,
        }
    }
}

#[derive(Debug, Clone, RlpEncodable, RlpDecodable)]
pub struct EnrResponse {
    pub request_hash: H256,
    pub enr: enr::Enr<secp256k1::SecretKey>,
}

impl EnrResponse {
    pub fn new(request_hash: H256, enr: enr::Enr<secp256k1::SecretKey>) -> Self {
        Self { request_hash, enr }
    }
    /// Returns the [`ForkId`] if set
    ///
    /// See also <https://github.com/ethereum/go-ethereum/blob/9244d5cd61f3ea5a7645fdf2a1a96d53421e412f/eth/protocols/eth/discovery.go#L36>
    pub fn eth_fork_id(&self) -> Option<ForkId> {
        let mut maybe_fork_id = self.enr.get(b"eth")?;
        ForkId::decode(&mut maybe_fork_id).ok()
    }
}
