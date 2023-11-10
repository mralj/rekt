use open_fastrlp::DecodeError;
use thiserror::Error;

use crate::eth::transactions::errors::DecodeTxError;

#[derive(Debug, Error, Copy, Clone)]
pub enum ETHError {
    #[error(transparent)]
    RLPDecoding(#[from] DecodeError),
    #[error(transparent)]
    TxDecoding(#[from] DecodeTxError),
}
