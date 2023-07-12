use serde::{Deserialize, Serialize};

use crate::types::hash::H256;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Head {
    /// The number of the head block.
    pub number: u64,
    /// The hash of the head block.
    pub hash: H256,
    /// The difficulty of the head block.
    pub difficulty: u64,
    /// The total difficulty at the head block.
    pub total_difficulty: u64,
    /// The timestamp of the head block.
    pub timestamp: u64,
}
