use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::local_node::LocalNode;
use crate::types::hash::H256;
use crate::types::node_record::NodeRecord;

use super::discover_message::DEFAULT_MESSAGE_EXPIRATION;

// Defined by the docs, this is hardcoded
// https://github.com/ethereum/devp2p/blob/master/discv4.md
// 4 is for discovery v4
const DEFAULT_IP_PACKET_V: u8 = 4;

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

impl From<&NodeRecord> for Endpoint {
    fn from(node_record: &NodeRecord) -> Self {
        Self {
            ip: node_record.address,
            udp: node_record.udp_port,
            tcp: node_record.tcp_port,
        }
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

impl PingMessage {
    pub fn new(our_node: &LocalNode, target_node: &NodeRecord) -> Self {
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + DEFAULT_MESSAGE_EXPIRATION;

        PingMessage {
            version: DEFAULT_IP_PACKET_V,
            from: Endpoint::from(&our_node.node_record),
            to: Endpoint::from(target_node),
            expiration: expires,
            enr_seq: our_node.enr.seq(),
        }
    }
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
