use bytes::{Buf, BufMut};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use open_fastrlp::{Decodable, DecodeError, Encodable};

use super::HelloMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum P2PMessage {
    /// The first packet sent over the connection, and sent once by both sides.
    Hello(HelloMessage),

    /// Inform the peer that a disconnection is imminent; if received, a peer should disconnect
    /// immediately.
    Disconnect,

    /// Requests an immediate reply of [`P2PMessage::Pong`] from the peer.
    Ping,

    /// Reply to the peer's [`P2PMessage::Ping`] packet.
    Pong,
}

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum P2PMessageID {
    Hello = 0x00,
    Disconnect = 0x01,
    Ping = 0x02,
    Pong = 0x03,
}

impl Decodable for P2PMessageID {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let message_id = u8::decode(&mut &buf[..])?;
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
