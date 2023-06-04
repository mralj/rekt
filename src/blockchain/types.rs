use std::str::FromStr;
use std::sync::OnceLock;

use ethers::types::U256;
use serde::{Deserialize, Serialize};

use crate::types::hash::H256;

pub static BSC_CHAIN: OnceLock<ChainConfig> = OnceLock::new();

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChainConfig {
    /// The network's chain ID.
    pub chain_id: u64,

    pub td: u64,
    pub genesis_hash: H256,

    /// The homestead switch block (None = no fork, 0 = already homestead).
    pub homestead_block: Option<u64>,

    /// The DAO fork switch block (None = no fork).
    pub dao_fork_block: Option<u64>,

    /// Whether or not the node supports the DAO hard-fork.
    pub dao_fork_support: bool,

    /// The EIP-150 hard fork block (None = no fork).
    pub eip150_block: Option<u64>,

    /// The EIP-150 hard fork hash.
    pub eip150_hash: Option<H256>,

    /// The EIP-155 hard fork block.
    pub eip155_block: Option<u64>,

    /// The EIP-158 hard fork block.
    pub eip158_block: Option<u64>,

    /// The Byzantium hard fork block.
    pub byzantium_block: Option<u64>,

    /// The Constantinople hard fork block.
    pub constantinople_block: Option<u64>,

    /// The Petersburg hard fork block.
    pub petersburg_block: Option<u64>,

    /// The Istanbul hard fork block.
    pub istanbul_block: Option<u64>,

    /// The Muir Glacier hard fork block.
    pub muir_glacier_block: Option<u64>,

    pub ramanujan_block: Option<u64>,
    pub niels_block: Option<u64>,
    pub mirror_sync_block: Option<u64>,
    pub bruno_block: Option<u64>,
    pub euler_block: Option<u64>,
    pub nano_block: Option<u64>,
    pub moran_block: Option<u64>,
    pub gibbs_block: Option<u64>,
    pub planck_block: Option<u64>,
    pub luban_block: Option<u64>,
    pub plato_block: Option<u64>,

    pub parlia: Option<ParliaConfig>,
}

/// Empty consensus configuration for proof-of-work networks.
/// Consensus configuration for Clique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParliaConfig {
    /// Number of seconds between blocks to enforce.
    pub period: Option<u64>,

    /// Epoch length to reset votes and checkpoints.
    pub epoch: Option<u64>,
}

impl ChainConfig {
    pub fn new_bsc_chain() -> &'static Self {
        BSC_CHAIN.get_or_init(|| Self {
            chain_id: 56,
            td: 1,
            genesis_hash: H256::from_str(
                "0x0d21840abff46b96c84b2ac9e10e4f5cdaeb5693cb665db62a2f3b02d2d57b5b",
            )
            .unwrap(),
            homestead_block: None,
            dao_fork_block: None,
            dao_fork_support: false,
            eip150_block: None,
            eip150_hash: None,
            eip155_block: None,
            eip158_block: None,
            byzantium_block: None,
            constantinople_block: None,
            petersburg_block: None,
            istanbul_block: None,
            muir_glacier_block: None,
            niels_block: None,
            ramanujan_block: None,
            mirror_sync_block: Some(5184000),
            bruno_block: Some(13082000),
            euler_block: Some(18907621),
            nano_block: Some(21962149),
            moran_block: Some(22107423),
            gibbs_block: Some(23846001),
            planck_block: Some(27281024),
            luban_block: Some(29020050),
            plato_block: None,
            parlia: Some(ParliaConfig {
                period: Some(3),
                epoch: Some(200),
            }),
        })
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Head {
    /// The number of the head block.
    pub number: u64,
    /// The hash of the head block.
    pub hash: H256,
    /// The difficulty of the head block.
    pub difficulty: U256,
    /// The total difficulty at the head block.
    pub total_difficulty: U256,
    /// The timestamp of the head block.
    pub timestamp: u64,
}
