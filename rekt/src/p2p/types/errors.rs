use thiserror::Error;

use crate::p2p::DisconnectReason;

#[derive(Debug, Error, Copy, Clone)]
pub enum P2PError {
    #[error("No message")]
    NoMessage,
    #[error("Decode error: {0}")]
    MessageDecodeError(#[from] open_fastrlp::DecodeError),
    #[error("Could not decode message id")]
    MessageIdDecodeError,
    #[error("Could not decode message kind")]
    MessageKindDecodeError,
    #[error("Snappy compress error")]
    SnappyCompressError,
    #[error("Too many messages queued")]
    TooManyMessagesQueued,
    #[error("RLPX error")]
    RlpxError,
    #[error("Disconnect requested: {0}")]
    DisconnectRequested(DisconnectReason),
    #[error("Unexpected hello message received")]
    UnexpectedHelloMessageReceived,
    #[error("Expected status message")]
    ExpectedStatusMessage,
    #[error("Expected upgrade status message")]
    ExpectedUpgradeStatusMessage,
    #[error("Could not validate status message")]
    CouldNotValidateStatusMessage,
    #[error("Too many attempts")]
    TooManyConnectionAttempts,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("Already connected to the same ip")]
    AlreadyConnectedToSameIp,
}
