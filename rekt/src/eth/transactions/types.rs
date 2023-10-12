use derive_more::Display;

use super::errors::DecodeTxError;

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub(super) enum TxType {
    Legacy,
    AccessList,
    DynamicFee,
    Blob,
}

impl TryFrom<u8> for TxType {
    type Error = DecodeTxError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(TxType::Legacy),
            0x01 => Ok(TxType::AccessList),
            0x02 => Ok(TxType::DynamicFee),
            0x03 => Ok(TxType::Blob),
            _ => Err(DecodeTxError::UnknownTxType),
        }
    }
}
