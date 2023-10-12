use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeTxError {
    #[error("Decode error: {0}")]
    MessageDecodeError(#[from] open_fastrlp::DecodeError),
    #[error("TX is contract creation")]
    ContractCreation,
    #[error("TX is of unknown type")]
    UnknownTxType,
}
