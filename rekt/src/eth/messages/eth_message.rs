use bytes::{Bytes, BytesMut};
use open_fastrlp::DecodeError;

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
}

impl EthMessage {
    pub fn new(id: EthProtocol, data: Bytes) -> Self {
        Self {
            id,
            data,
            compressed: EthMessageCompressionStatus::Uncompressed,
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
        }
    }

    pub fn new_devp2p_ping_message() -> Self {
        Self {
            data: Bytes::new(),
            id: EthProtocol::DevP2PPing,
            compressed: EthMessageCompressionStatus::Compressed,
        }
    }

    pub fn is_compressed(&self) -> bool {
        self.compressed == EthMessageCompressionStatus::Compressed
    }

    pub fn snappy_decompress(
        &mut self,
        snappy_decoder: &mut snap::raw::Decoder,
    ) -> Result<(), DecodeError> {
        let decompressed_len = snap::raw::decompress_len(&self.data)
            .map_err(|_| DecodeError::Custom("Could not read length for snappy decompress"))?;
        let mut rlp_msg_bytes = BytesMut::zeroed(decompressed_len);

        snappy_decoder
            .decompress(&self.data, &mut rlp_msg_bytes)
            .map_err(|_| DecodeError::Custom("Could not snap decompress msg"))?;

        self.data = rlp_msg_bytes.freeze();

        Ok(())
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
