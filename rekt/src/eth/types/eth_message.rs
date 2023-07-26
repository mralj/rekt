use bytes::BytesMut;

use crate::eth::protocol::EthMessages;
use crate::p2p::types::p2p_wire_message::P2pWireMessage;

pub struct EthMessage {
    pub id: EthMessages,
    pub data: BytesMut,
}

impl From<P2pWireMessage> for EthMessage {
    fn from(msg: P2pWireMessage) -> Self {
        Self {
            id: EthMessages::from(msg.id),
            data: msg.data,
        }
    }
}
