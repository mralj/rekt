use bytes::{Buf, BufMut};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use open_fastrlp::{Decodable, DecodeError, Encodable};

use crate::p2p::DisconnectReason;

use super::HelloMessage;

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum P2PMessageID {
    Hello = 0x00,
    Disconnect = 0x01,
    Ping = 0x02,
    Pong = 0x03,
}

impl Decodable for P2PMessageID {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let message_id = u8::decode(&mut &buf[..1])?;
        let id = P2PMessageID::from_u8(message_id).ok_or(DecodeError::Custom("Invalid msg ID"))?;

        buf.advance(1);
        Ok(id)
    }
}

impl Encodable for P2PMessageID {
    fn encode(&self, out: &mut dyn BufMut) {
        self.to_u8().unwrap().encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageID {
    P2PMessageID(P2PMessageID),
    CapabilityMessageId(u8),
}

impl Decodable for MessageID {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let maybe_p2p_msg = P2PMessageID::decode(buf);
        if !maybe_p2p_msg.is_err() {
            return Ok(MessageID::P2PMessageID(maybe_p2p_msg.unwrap()));
        }

        let message_id = u8::decode(&mut &buf[..1])?;
        buf.advance(1);

        match message_id {
            0x0..=0x0f => Err(DecodeError::Custom(
                "Capability message cannot be less than 0x10, dec: 16",
            )),
            0x10..=0x20 => Ok(MessageID::CapabilityMessageId(message_id)),
            _ => Err(DecodeError::Custom("Capability message  id incorrect")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum P2PMessage {
    /// The first packet sent over the connection, and sent once by both sides.
    Hello(HelloMessage),

    /// Inform the peer that a disconnection is imminent; if received, a peer should disconnect
    /// immediately.
    Disconnect(DisconnectReason),

    /// Requests an immediate reply of [`P2PMessage::Pong`] from the peer.
    Ping,

    /// Reply to the peer's [`P2PMessage::Ping`] packet.
    Pong,
}
impl P2PMessage {
    pub fn decode(id: P2PMessageID, buf: &mut &[u8]) -> Result<Self, DecodeError> {
        match id {
            P2PMessageID::Hello => Ok(P2PMessage::Hello(HelloMessage::decode(buf)?)),
            P2PMessageID::Disconnect => Ok(P2PMessage::Disconnect(DisconnectReason::decode(buf)?)),
            P2PMessageID::Ping => {
                //TODO: do we really have to advance the buffer here?
                // what happens if we don't?
                buf.advance(1);
                Ok(P2PMessage::Ping)
            }
            P2PMessageID::Pong => {
                //TODO: do we really have to advance the buffer here?
                // what happens if we don't?
                buf.advance(1);
                Ok(P2PMessage::Pong)
            }
        }
    }
}
