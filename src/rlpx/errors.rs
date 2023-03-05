use open_fastrlp::DecodeError;
use thiserror::Error;

use super::connection::RLPXConnectionState;

#[derive(Debug, Error)]
pub enum RLPXError {
    /// Error when parsing ACK data
    #[error("invalid ack data")]
    InvalidAckData,
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
    #[error(transparent)]
    DecodeError(#[from] std::io::Error),
    #[error("Received unexpected message: {received} when expecting {expected}")]
    UnexpectedMessage {
        received: RLPXConnectionState,
        expected: RLPXConnectionState,
    },
}
