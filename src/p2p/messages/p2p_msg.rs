use bytes::BufMut;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use open_fastrlp::{Decodable, DecodeError, Encodable};

use super::{DisconnectReason, HelloMessage};

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum P2PMessageID {
    Hello = 0x00,
    Disconnect = 0x01,
    Ping = 0x02,
    Pong = 0x03,
}

impl Encodable for P2PMessageID {
    fn encode(&self, out: &mut dyn BufMut) {
        self.to_u8().unwrap().encode(out);
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
    pub fn decode(id: u8, buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let id = P2PMessageID::from_u8(id).ok_or(DecodeError::Custom("Invalid message id"))?;
        match id {
            P2PMessageID::Hello => Ok(P2PMessage::Hello(HelloMessage::decode(buf)?)),
            P2PMessageID::Disconnect => Ok(P2PMessage::Disconnect(DisconnectReason::decode(buf)?)),
            P2PMessageID::Ping => Ok(P2PMessage::Ping),
            P2PMessageID::Pong => Ok(P2PMessage::Pong),
        }
    }
}
