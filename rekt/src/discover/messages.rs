use std::net::IpAddr;

use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Endpoint {
    pub(super) ip: IpAddr,
    pub(super) udp: u16,
    pub(super) tcp: u16,
}

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct PingMessage {
    pub(super) version: u8,
    pub(super) from: Endpoint,
    pub(super) to: Endpoint,
    pub(super) expiration: u64,
    //pub(super) rest: Option<u8>,
}
