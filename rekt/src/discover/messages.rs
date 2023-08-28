use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::{Bytes, BytesMut};
use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::types::hash::H256;

const DEFAULT_PONG_EXPIRATION: u64 = 20;

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

// pub struct Pong {
//     pub to: NodeEndpoint,
//     pub echo: H256,
//     pub expire: u64,
// }

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
            + DEFAULT_PONG_EXPIRATION;

        Self {
            hash,
            expires,
            to: ping_msg.to,
        }
    }

    pub fn rlp_encode(&self) -> Bytes {
        let mut rlp_encoded_msg = BytesMut::new();
        self.encode(&mut rlp_encoded_msg);
        rlp_encoded_msg.freeze()
    }
}
