use open_fastrlp::DecodeError;
use thiserror::Error;

#[derive(Debug, Error, Copy, Clone)]
pub enum ETHError {
    #[error(transparent)]
    RLPDecoding(#[from] DecodeError),
}
