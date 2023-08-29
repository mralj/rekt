use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::types::hash::H256;

use super::discover_message::DEFAULT_MESSAGE_EXPIRATION;

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Endpoint {
    pub(super) ip: IpAddr,
    pub(super) udp: u16,
    pub(super) tcp: u16,
}

impl Endpoint {
    pub fn new(ip: IpAddr, udp: u16, tcp: u16) -> Self {
        Self { ip, udp, tcp }
    }
}

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct PingMessage {
    pub(super) version: u8,
    pub(super) from: Endpoint,
    pub(super) to: Endpoint,
    pub(super) expiration: u64,
    pub(super) enr_seq: u64,
}

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct PongMessage {
    pub(super) to: Endpoint,
    pub(super) hash: H256,
    pub(super) expires: u64,
}

impl PongMessage {
    pub fn new(ping_msg: PingMessage, hash: H256) -> Self {
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + DEFAULT_MESSAGE_EXPIRATION;

        Self {
            hash,
            expires,
            to: ping_msg.to,
        }
    }
}
