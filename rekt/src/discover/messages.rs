use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Endpoint {
    pub(super) ip: String,
    pub(super) udp: u16,
    pub(super) tcp: u16,
}

impl Endpoint {
    pub fn new(ip: String, udp: u16, tcp: u16) -> Self {
        Self { ip, udp, tcp }
    }
}

#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct PingMessage {
    pub(super) version: u8,
    pub(super) from: Endpoint,
    pub(super) to: Endpoint,
    pub(super) expiration: u64,
}
