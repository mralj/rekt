use bytes::{Buf, BytesMut};
use derive_more::Display;
use open_fastrlp::{Decodable, DecodeError};

use crate::eth::protocol::MAX_ETH_PROTOCOL_LEN;

// when we receive message data, the first byte will be the message id
// and the rest will be actual data
const POSITION_OF_MSG_ID_IN_BYTE_BUFFER: usize = 1;
const BASE_PROTOCOL_OFFSET: u8 = 16;
const MAX_SUPPORTED_MESSAGE_ID: u8 = BASE_PROTOCOL_OFFSET + MAX_ETH_PROTOCOL_LEN;

#[derive(Debug, Display, Clone, Eq, PartialEq)]
pub enum MessageKind {
    P2P,
    ETH,
}

#[derive(Debug, Clone)]
pub struct P2pWireMessage {
    pub kind: MessageKind,
    pub id: u8,
    pub data: BytesMut,
}

impl P2pWireMessage {
    pub fn new(mut data: BytesMut) -> Result<P2pWireMessage, DecodeError> {
        let id = Self::decode_id(&mut &data[..])?;
        let kind = Self::decode_kind(id)?;

        // after we decoded id, the byte buffer has to move forwards for 1
        // because id was decoded, and we'll have to decode the rest of the message
        data.advance(POSITION_OF_MSG_ID_IN_BYTE_BUFFER);
        Ok(P2pWireMessage { kind, id, data })
    }

    fn decode_kind(id: u8) -> Result<MessageKind, DecodeError> {
        match id {
            0x0..=0x03 => Ok(MessageKind::P2P),
            BASE_PROTOCOL_OFFSET..=MAX_SUPPORTED_MESSAGE_ID => Ok(MessageKind::ETH),
            _ => Err(DecodeError::Custom("Decoded message id out of bounds")),
        }
    }

    fn decode_id(data: &mut &[u8]) -> Result<u8, DecodeError> {
        // Just in case this was unintentionally called twice
        let message_id = u8::decode(&mut &data[..POSITION_OF_MSG_ID_IN_BYTE_BUFFER])
            .map_err(|_| DecodeError::Custom("Invalid message id"))?;

        match message_id {
            0x0..=MAX_SUPPORTED_MESSAGE_ID => Ok(message_id),
            _ => Err(DecodeError::Custom("Decoded message id out of bounds")),
        }
    }

    pub fn snappy_decompress(
        &mut self,
        snappy_decoder: &mut snap::raw::Decoder,
    ) -> Result<(), DecodeError> {
        // we skip decompressing p2p messages, except Disconnect
        match self.id {
            0x00 => return Ok(()),
            0x02 => return Ok(()),
            0x03 => return Ok(()),
            _ => {}
        }

        let decompressed_len = snap::raw::decompress_len(&self.data)
            .map_err(|_| DecodeError::Custom("Could not read length for snappy decompress"))?;
        let mut rlp_msg_bytes = BytesMut::zeroed(decompressed_len);

        snappy_decoder
            .decompress(&self.data, &mut rlp_msg_bytes)
            .map_err(|_| DecodeError::Custom("Could not snap decompress msg"))?;

        self.data = rlp_msg_bytes;

        Ok(())
    }
}
