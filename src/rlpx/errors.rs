use std::num::TryFromIntError;

use open_fastrlp::DecodeError;
use thiserror::Error;

use super::codec::RLPXMsg;
use crate::p2p;
use crate::server::connection_task::ConnectionTask;

#[derive(Debug, Error)]
pub enum RLPXError {
    /// Error when parsing ACK data
    #[error("invalid ack data")]
    InvalidAckData,
    #[error("invalid msg data")]
    InvalidMsgData,
    /// Error when checking the HMAC tag against the tag on the message being decrypted
    #[error("tag check failure in read_header")]
    TagCheckDecryptFailed,
    /// Error when interacting with secp256k1
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
    /// Error when decoding RLP data
    #[error(transparent)]
    RLPDecoding(#[from] DecodeError),
    /// Error when trying to split an array beyond its length
    #[error("requested {idx} but array len is {len}")]
    OutOfBounds {
        /// The index you are trying to split at
        idx: usize,
        /// The length of the array
        len: usize,
    },
    #[error("Decoding error during RLPX: {0}")]
    DecodeError(String),
    #[error("Received unexpected message: {received} when expecting {expected}")]
    UnexpectedMessage {
        received: RLPXMsg,
        expected: RLPXMsg,
    },
    #[error("Invalid header")]
    InvalidHeader,
    /// Error when checking the HMAC tag against the tag on the header
    #[error("tag check failure in read_header")]
    TagCheckHeaderFailed,
    /// Error when checking the HMAC tag against the tag on the body
    #[error("tag check failure in read_body")]
    TagCheckBodyFailed,
}

impl From<std::io::Error> for RLPXError {
    fn from(error: std::io::Error) -> Self {
        RLPXError::DecodeError(format!("IO error: {}", error))
    }
}

impl From<std::array::TryFromSliceError> for RLPXError {
    fn from(error: std::array::TryFromSliceError) -> Self {
        RLPXError::DecodeError(format!("Slice conversion error: {}", error))
    }
}

impl From<TryFromIntError> for RLPXError {
    fn from(error: TryFromIntError) -> Self {
        RLPXError::DecodeError(format!("Int conversion error: {}", error))
    }
}

#[derive(Debug, Error)]
pub enum RLPXSessionError {
    #[error("Unknown Error")]
    UnknownError,
    #[error("No message received from TCP stream")]
    NoMessage,
    #[error("RLPX error: {0}")]
    RlpxError(#[from] RLPXError),
    #[error("TCP IO error: {0}")]
    TcpError(#[from] std::io::Error),
    #[error("Expected RLPX Message, but received Auth/Ack/smth. random.")]
    ExpectedRLPXMessage,
    #[error("Unexpected RLPX message: {received} when expecting {expected}")]
    UnexpectedMessage {
        received: RLPXMsg,
        expected: RLPXMsg,
    },
    #[error("Unexpected message ID")]
    UnexpectedMessageID {
        received: u8,
        expected: p2p::P2PMessageID,
    },
    #[error("Unexpected message: {received} when expecting {expected}")]
    UnexpectedP2PMessage { received: u8, expected: u8 },
    #[error("Decode error: {0}")]
    MessageDecodeError(#[from] open_fastrlp::DecodeError),
    #[error("Disconnect requested: {0}")]
    DisconnectRequested(p2p::DisconnectReason),
    #[error("No matching protocols found")]
    NoMatchingProtocols,
    #[error("Unsupported protocol version: {0}")]
    UnsupportedProtocol(#[from] p2p::types::protocol::ProtocolVersionError),
}

pub struct PeerErr {
    pub conn_task: ConnectionTask,
    pub err: RLPXSessionError,
}

impl PeerErr {
    pub fn new(conn_task: ConnectionTask, err: RLPXSessionError) -> Self {
        PeerErr { conn_task, err }
    }
}
