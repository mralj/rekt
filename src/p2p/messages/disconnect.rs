use derive_more::Display;
use open_fastrlp::{Decodable, DecodeError, Header};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display)]
pub enum DisconnectReason {
    #[default]
    DisconnectRequested,
    TcpSubsystemError,
    ProtocolBreach,
    UselessPeer,
    TooManyPeers,
    AlreadyConnected,
    IncompatibleP2PProtocolVersion,
    NullNodeIdentity,
    ClientQuitting,
    UnexpectedHandshakeIdentity,
    ConnectedToSelf,
    PingTimeout,
    SubprotocolSpecific,
    Unknown,
}

#[derive(Debug, Clone, Error)]
#[error("unknown disconnect reason: {0}")]
pub struct UnknownDisconnectReason(u8);

impl From<u8> for DisconnectReason {
    fn from(value: u8) -> Self {
        match value {
            0x00 => DisconnectReason::DisconnectRequested,
            // THIS IS GETH idiosyncrasy, 0x80 is empty string, which GETH sends as a disconnect reason
            0x80 => DisconnectReason::DisconnectRequested,
            0x01 => DisconnectReason::TcpSubsystemError,
            0x02 => DisconnectReason::ProtocolBreach,
            0x03 => DisconnectReason::UselessPeer,
            0x04 => DisconnectReason::TooManyPeers,
            0x05 => DisconnectReason::AlreadyConnected,
            0x06 => DisconnectReason::IncompatibleP2PProtocolVersion,
            0x07 => DisconnectReason::NullNodeIdentity,
            0x08 => DisconnectReason::ClientQuitting,
            0x09 => DisconnectReason::UnexpectedHandshakeIdentity,
            0x0a => DisconnectReason::ConnectedToSelf,
            0x0b => DisconnectReason::PingTimeout,
            0x10 => DisconnectReason::SubprotocolSpecific,
            _ => DisconnectReason::Unknown,
        }
    }
}

impl Decodable for DisconnectReason {
    //NOTE: the message ID is already parsed, this parses the message body
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        loop {
            match buf.len() {
                0 => return Err(DecodeError::InputTooShort),
                1 => return Ok(DisconnectReason::from(buf[0])),
                2 => {
                    let header = Header::decode(buf)?;
                    if !header.list {
                        return Err(DecodeError::UnexpectedString);
                    }
                }
                _ => return Err(DecodeError::Overflow),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::p2p::messages::p2p_msg::P2PMessage;

    use super::*;

    #[test]
    fn test_reason_too_short() {
        assert!(DisconnectReason::decode(&mut &[0u8; 0][..]).is_err())
    }

    #[test]
    fn test_reason_too_long() {
        assert!(DisconnectReason::decode(&mut &[0u8; 3][..]).is_err())
    }

    #[test]
    fn test_decode_known_reasons() {
        let all_reasons = vec![
            // encoding the disconnect reason as a single byte
            "0100", // 0x00 case
            "0180", // second 0x00 case
            "0101", "0102", "0103", "0104", "0105", "0106", "0107", "0108", "0109", "010a", "010b",
            "0110",   // encoding the disconnect reason in a list
            "01c100", // 0x00 case
            "01c180", // second 0x00 case
            "01c101", "01c102", "01c103", "01c104", "01c105", "01c106", "01c107", "01c108",
            "01c109", "01c10a", "01c10b", "01c110",
        ];

        for reason in all_reasons {
            let reason = hex::decode(reason).unwrap();
            let message =
                P2PMessage::decode(crate::p2p::P2PMessageID::Disconnect, &mut &reason[1..])
                    .unwrap();
            let P2PMessage::Disconnect(_) = message else {
                panic!("expected a disconnect message");
            };
        }
    }

    #[test]
    fn test_decode_disconnect_requested() {
        let reason = "0100";
        let reason = hex::decode(reason).unwrap();
        match P2PMessage::decode(crate::p2p::P2PMessageID::Disconnect, &mut &reason[1..]).unwrap() {
            P2PMessage::Disconnect(DisconnectReason::DisconnectRequested) => {}
            _ => {
                unreachable!()
            }
        }
    }
}
