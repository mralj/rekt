use std::time::{SystemTime, UNIX_EPOCH};

use open_fastrlp::{RlpDecodable, RlpEncodable};

use super::discover_message::DEFAULT_MESSAGE_EXPIRATION;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

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
    pub nodes: Vec<NodeRecord>,
    pub expire: u64,
}
