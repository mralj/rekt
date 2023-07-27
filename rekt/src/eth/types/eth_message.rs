use bytes::BytesMut;

use crate::eth::protocol::EthMessages;
use crate::p2p::types::p2p_wire_message::P2pWireMessage;

pub const BASE_PROTOCOL_OFFSET: u8 = 16;

#[derive(Debug, Clone)]
pub struct EthMessage {
    pub id: EthMessages,
    pub data: BytesMut,
}

impl EthMessage {
    pub fn new(id: EthMessages, data: BytesMut) -> Self {
        Self { id, data }
    }
}

impl From<P2pWireMessage> for EthMessage {
    fn from(msg: P2pWireMessage) -> Self {
        let id = msg.id - BASE_PROTOCOL_OFFSET;
        Self {
            id: EthMessages::from(id),
            data: msg.data,
        }
    }
}
