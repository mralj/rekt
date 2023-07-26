use derive_more::Display;

pub const MAX_ETH_PROTOCOL_LEN: u8 = 18;

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
pub enum EthMessages {
    StatusMsg = 0x00,
    NewBlockHashesMsg = 0x01,
    TransactionsMsg = 0x02,
    GetBlockHeadersMsg = 0x03,
    BlockHeadersMsg = 0x04,
    GetBlockBodiesMsg = 0x05,
    BlockBodiesMsg = 0x06,
    NewBlockMsg = 0x07,
    GetNodeDataMsg = 0x0d,
    NodeDataMsg = 0x0e,
    GetReceiptsMsg = 0x0f,
    ReceiptsMsg = 0x10,
    NewPooledTransactionHashesMsg = 0x08,
    GetPooledTransactionsMsg = 0x09,
    PooledTransactionsMsg = 0x0a,
    // Protocol messages overloaded in eth/67
    UpgradeStatusMsg = 0x0b,
    Unknown = 0xff,
}

impl From<u8> for EthMessages {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::StatusMsg,
            0x01 => Self::NewBlockHashesMsg,
            0x02 => Self::TransactionsMsg,
            0x03 => Self::GetBlockHeadersMsg,
            0x04 => Self::BlockHeadersMsg,
            0x05 => Self::GetBlockBodiesMsg,
            0x06 => Self::BlockBodiesMsg,
            0x07 => Self::NewBlockMsg,
            0x0d => Self::GetNodeDataMsg,
            0x0e => Self::NodeDataMsg,
            0x0f => Self::GetReceiptsMsg,
            0x10 => Self::ReceiptsMsg,
            0x08 => Self::NewPooledTransactionHashesMsg,
            0x09 => Self::GetPooledTransactionsMsg,
            0x0a => Self::PooledTransactionsMsg,
            0x0b => Self::UpgradeStatusMsg,
            _ => Self::Unknown,
        }
    }
}
