use bytes::{Buf, BytesMut};
use derive_more::Display;
use open_fastrlp::{Decodable, DecodeError};

use crate::p2p::P2PMessage;

// when we receive message data, the first byte will be the message id
// and the rest will be actual data
const POSITION_OF_MSG_ID_IN_BYTE_BUFFER: usize = 1;
//TODO: handle this better once we introduce ETH protocol
// for the time being the explanation is as follows:
// first 16 message ids ([0,15]) are reserved for P2P
// ETH message have IDs 16 onward, and ATM there is 16 message types
const MAX_SUPPORTED_MESSAGE_ID: u8 = 32;

#[derive(Debug, Display, Clone, Eq, PartialEq)]
pub enum MessageKind {
    P2P(P2PMessage),
    ETH,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub kind: Option<MessageKind>,
    pub id: Option<u8>,
    pub data: BytesMut,
}

impl Message {
    pub fn new(data: BytesMut) -> Self {
        Message {
            id: None,
            kind: None,
            data,
        }
    }

    pub fn decode_id(&mut self) -> Result<u8, DecodeError> {
        // Just in case this was unintentionally called twice
        if self.id.is_some() {
            return Ok(self.id.unwrap());
        }

        let message_id = u8::decode(&mut &self.data[..POSITION_OF_MSG_ID_IN_BYTE_BUFFER])
            .map_err(|_| DecodeError::Custom("Invalid message id"))?;

        match message_id {
            0x0..=MAX_SUPPORTED_MESSAGE_ID => {
                // after we decoded id, the byte buffer has to move forwards for 1
                // because id was decoded, and we'll have to decode the rest of the message
                self.data.advance(POSITION_OF_MSG_ID_IN_BYTE_BUFFER);
                self.id = Some(message_id);
                Ok(message_id)
            }
            _ => Err(DecodeError::Custom("Decoded message id out of bounds")),
        }
    }

    pub fn decode_kind(&mut self) -> Result<&MessageKind, DecodeError> {
        // Just in case this was unintentionally called twice
        if self.kind.is_some() {
            return Ok(self.kind.as_ref().unwrap());
        }

        if self.id.is_none() {
            return Err(DecodeError::Custom(
                "Cannot decode message if ID is invalid",
            ));
        }

        let id = self.id.unwrap();
        match id {
            // 0x00 Hello
            // 0x01 Disconnect
            // 0x02 Ping
            // 0x03 Pong
            // I tried to derive from P2PMessageId enum this  but I could not find sane way
            // So leaving this as is
            0x0..=0x03 => {
                let p2p_msg = P2PMessage::decode(id, &mut &self.data[..])?;
                self.kind = Some(MessageKind::P2P(p2p_msg));
            }
            0x10..=MAX_SUPPORTED_MESSAGE_ID => self.kind = Some(MessageKind::ETH),
            _ => return Err(DecodeError::Custom("Decoded message id out of bounds")),
        }

        Ok(self.kind.as_ref().unwrap())
    }

    pub fn snappy_decompress(
        &mut self,
        snappy_decoder: &mut snap::raw::Decoder,
    ) -> Result<(), DecodeError> {
        let msg_is_ping_pong_no_need_to_decompress = self.id == Some(0x02) || self.id == Some(0x03);
        if msg_is_ping_pong_no_need_to_decompress {
            return Ok(());
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
