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

#[derive(Debug, Display)]
pub enum MessageKind {
    Unknown,
    P2P(P2PMessage),
    ETH,
}

#[derive(Debug)]
pub struct Message {
    pub(crate) kind: MessageKind,
    pub(crate) data: BytesMut,
    pub(crate) id: Option<u8>,
}

impl Message {
    pub fn new(data: BytesMut) -> Self {
        Message {
            id: None,
            kind: MessageKind::Unknown,
            data,
        }
    }

    pub fn decode_id(&mut self) -> Result<u8, DecodeError> {
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

    pub fn decode_kind(&mut self) -> Result<(), DecodeError> {
        if self.id.is_none() {
            return Err(DecodeError::Custom(
                "Cannot decode message if ID is invalid",
            ));
        }

        let id = self.id.unwrap();
        match id {
            0x0..=0x03 => {
                let p2p_msg = P2PMessage::decode(id, &mut &self.data[..])?;
                self.kind = MessageKind::P2P(p2p_msg);
            }
            _ => self.kind = MessageKind::ETH,
        }

        Ok(())
    }
}
