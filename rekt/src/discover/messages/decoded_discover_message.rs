use std::net::SocketAddr;

use crate::types::hash::{H256, H512};

use super::discover_message::DiscoverMessage;

pub struct DecodedDiscoverMessage {
    pub(crate) msg: DiscoverMessage,
    pub(crate) node_id: H512,
    pub(crate) hash: H256,
    pub(crate) from: SocketAddr,
}

impl DecodedDiscoverMessage {
    pub(crate) fn new(from: SocketAddr, msg: DiscoverMessage, node_id: H512, hash: &[u8]) -> Self {
        Self {
            from,
            msg,
            node_id,
            hash: H256::from_slice(hash),
        }
    }
}
