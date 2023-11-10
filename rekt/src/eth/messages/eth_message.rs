use bytes::Bytes;

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
    pub data: Bytes,
    pub compressed: EthMessageCompressionStatus,
    pub created_on: tokio::time::Instant,
}

impl EthMessage {
    pub fn new(id: EthProtocol, data: Bytes) -> Self {
        Self {
            id,
            data,
            compressed: EthMessageCompressionStatus::Uncompressed,
            created_on: tokio::time::Instant::now(),
        }
    }

    pub fn new_tx_message(data: Bytes) -> Self {
        Self::new(EthProtocol::TransactionsMsg, data)
    }

    pub fn new_compressed_tx_message(data: Bytes) -> Self {
        Self {
            data,
            id: EthProtocol::TransactionsMsg,
            compressed: EthMessageCompressionStatus::Compressed,
            created_on: tokio::time::Instant::now(),
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
            data: msg.data.freeze(),
            compressed: EthMessageCompressionStatus::Uncompressed,
            created_on: tokio::time::Instant::now(),
        }
    }
}
