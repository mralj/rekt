use bytes::BytesMut;

use crate::eth::types::protocol::{EthProtocol, ETH_PROTOCOL_OFFSET};
use crate::p2p::p2p_wire_message::P2pWireMessage;

#[derive(Debug, Clone)]
pub struct EthMessage {
    pub id: EthProtocol,
    pub data: BytesMut,
}

impl EthMessage {
    pub fn new(id: EthProtocol, data: BytesMut) -> Self {
        Self { id, data }
    }
}

impl From<P2pWireMessage> for EthMessage {
    fn from(msg: P2pWireMessage) -> Self {
        let id = msg.id - ETH_PROTOCOL_OFFSET;
        Self {
            id: EthProtocol::from(id),
            data: msg.data,
        }
    }
}
