use hex_literal::hex;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

use crate::types::hash::H256;

use super::chain_spec::ChainSpec;
use super::fork::{ForkFilter, ForkId};
use super::fork_condition::ForkCondition;
use super::hard_fork::Hardfork;

pub static BSC_MAINNET: Lazy<ChainSpec> = Lazy::new(|| {
    let genesis_hash = H256(hex!(
        "0d21840abff46b96c84b2ac9e10e4f5cdaeb5693cb665db62a2f3b02d2d57b5b"
    ));
    ChainSpec {
        chain: 56,
        td: 1,
        genesis_hash,
        head: super::head::Head {
            number: 0,
            hash: genesis_hash,
            difficulty: 1,
            total_difficulty: 1,
            timestamp: 1598664248,
        },
        hardforks: BTreeMap::from([
            (Hardfork::Homestead, ForkCondition::Block(0)),
            (Hardfork::Dao, ForkCondition::Block(0)),
            (Hardfork::Eip150, ForkCondition::Block(0)),
            (Hardfork::Eip155, ForkCondition::Block(0)),
            (Hardfork::Eip158, ForkCondition::Block(0)),
            (Hardfork::Byzantium, ForkCondition::Block(0)),
            (Hardfork::Constantinople, ForkCondition::Block(0)),
            (Hardfork::Petersburg, ForkCondition::Block(0)),
            (Hardfork::Istanbul, ForkCondition::Block(0)),
            (Hardfork::MuirGlacier, ForkCondition::Block(0)),
            (Hardfork::Niels, ForkCondition::Block(0)),
            (Hardfork::Ramanujan, ForkCondition::Block(0)),
            (Hardfork::MirrorSync, ForkCondition::Block(5184000)),
            (Hardfork::Bruno, ForkCondition::Block(13082000)),
            (Hardfork::Euler, ForkCondition::Block(18907621)),
            (Hardfork::Nano, ForkCondition::Block(21962149)),
            (Hardfork::Moran, ForkCondition::Block(22107423)),
            (Hardfork::Gibbs, ForkCondition::Block(23846001)),
            (Hardfork::Planck, ForkCondition::Block(27281024)),
            (Hardfork::Luban, ForkCondition::Block(29020050)),
            (Hardfork::Plato, ForkCondition::Never),
        ]),
    }
});

pub static BSC_MAINNET_FORK_ID: Lazy<ForkId> = Lazy::new(|| BSC_MAINNET.fork_id(&BSC_MAINNET.head));

pub static BSC_MAINNET_FORK_FILTER: Lazy<ForkFilter> =
    Lazy::new(|| BSC_MAINNET.fork_filter(BSC_MAINNET.head));
