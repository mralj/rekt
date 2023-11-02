use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::{Buf, BufMut};
use open_fastrlp::{Decodable, DecodeError, Encodable, Header, RlpDecodable, RlpEncodable};
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
    pub(crate) ip: IpAddr,
    pub(crate) udp: u16,
    pub(crate) tcp: u16,
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
    pub(crate) from: Endpoint,
    pub(super) to: Endpoint,
    pub(super) expiration: u64,
    pub(super) enr_seq: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    pub(super) to: Endpoint,
    pub(super) hash: H256,
    pub(super) expires: u64,
    pub enr_sq: Option<u64>,
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
            enr_sq: None,
        }
    }
}

impl Encodable for PongMessage {
    fn encode(&self, out: &mut dyn BufMut) {
        #[derive(RlpEncodable)]
        struct PongMessage<'a> {
            to: &'a Endpoint,
            hash: &'a H256,
            expires: u64,
        }

        PongMessage {
            to: &self.to,
            hash: &self.hash,
            expires: self.expires,
        }
        .encode(out);
    }
}

impl Decodable for PongMessage {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let b = &mut &**buf;
        let rlp_head = Header::decode(b)?;
        if !rlp_head.list {
            return Err(DecodeError::UnexpectedString);
        }
        let started_len = b.len();
        let mut this = Self {
            to: Decodable::decode(b)?,
            hash: Decodable::decode(b)?,
            expires: Decodable::decode(b)?,
            enr_sq: None,
        };

        if b.has_remaining() {
            this.enr_sq = Some(Decodable::decode(b)?);
        }

        let consumed = started_len - b.len();
        if consumed > rlp_head.payload_length {
            return Err(DecodeError::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        let rem = rlp_head.payload_length - consumed;
        b.advance(rem);
        *buf = *b;

        Ok(this)
    }
}
