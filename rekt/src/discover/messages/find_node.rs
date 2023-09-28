use open_fastrlp::{RlpDecodable, RlpEncodable};
use std::{
    fmt::Display,
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::types::hash::H512;

use super::discover_message::DEFAULT_MESSAGE_EXPIRATION;

#[derive(Clone, Copy, Debug, Eq, PartialEq, RlpEncodable, RlpDecodable)]
pub struct FindNode {
    pub id: H512,
    pub expires: u64,
}

impl FindNode {
    pub fn new(id: H512) -> Self {
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + DEFAULT_MESSAGE_EXPIRATION;

        Self { id, expires }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, RlpEncodable, RlpDecodable)]
pub struct Neighbours {
    pub nodes: Vec<NeighborNodeRecord>,
    pub expire: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, RlpEncodable, RlpDecodable)]
pub struct NeighborNodeRecord {
    /// The Address of a node.
    pub address: IpAddr,
    /// TCP port of the port that accepts connections.
    pub tcp_port: u16,
    /// UDP discovery port.
    pub udp_port: u16,
    /// Public key of the discovery service
    pub id: H512,
}

impl Display for NeighborNodeRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}, IP: {:?}, TCP: {}, UDP: {}",
            self.id, self.address, self.tcp_port, self.udp_port
        )
    }
}
