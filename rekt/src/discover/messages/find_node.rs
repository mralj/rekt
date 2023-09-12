use std::time::{SystemTime, UNIX_EPOCH};

use open_fastrlp::{RlpDecodable, RlpEncodable};

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
