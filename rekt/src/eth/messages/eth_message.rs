use bytes::BytesMut;

use crate::eth::types::protocol::{EthProtocol, ETH_PROTOCOL_OFFSET};
use crate::p2p::p2p_wire_message::P2pWireMessage;

#[derive(Default, Clone, PartialEq, Debug)]
pub enum EthMessageCompressionStatus {
    #[default]
    Uncompressed,
    Compressed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EthMessage {
    pub id: EthProtocol,
    pub data: BytesMut,
    pub compressed: EthMessageCompressionStatus,
}

impl EthMessage {
    pub fn new(id: EthProtocol, data: BytesMut) -> Self {
        Self {
            id,
            data,
            compressed: EthMessageCompressionStatus::Uncompressed,
        }
    }

    pub fn new_tx_message(data: BytesMut) -> Self {
        Self::new(EthProtocol::TransactionsMsg, data)
    }

    pub fn new_compressed_tx_message(data: BytesMut) -> Self {
        Self {
            data,
            id: EthProtocol::TransactionsMsg,
            compressed: EthMessageCompressionStatus::Compressed,
        }
    }

    pub fn is_compressed(&self) -> bool {
        self.compressed == EthMessageCompressionStatus::Compressed
    }
}

impl From<P2pWireMessage> for EthMessage {
    fn from(msg: P2pWireMessage) -> Self {
        let id = msg.id - ETH_PROTOCOL_OFFSET;
        Self {
            id: EthProtocol::from(id),
            data: msg.data,
            compressed: EthMessageCompressionStatus::Uncompressed,
        }
    }
}
