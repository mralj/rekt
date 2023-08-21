use serde::{Deserialize, Serialize};

use super::chain_spec::ChainSpec;
use super::fork::ForkId;
use super::fork_condition::ForkCondition;

/// The name of an Ethereum hardfork.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Hardfork {
    /// Frontier.
    Frontier,
    /// Homestead.
    Homestead,
    /// The DAO fork.
    Dao,
    /// EIP-150
    Eip150,
    /// EIP-155
    Eip155,
    /// EIP-158
    Eip158,
    /// Tangerine.
    Tangerine,
    /// Spurious Dragon.
    SpuriousDragon,
    /// Byzantium.
    Byzantium,
    /// Constantinople.
    Constantinople,
    /// Petersburg.
    Petersburg,
    /// Istanbul.
    Istanbul,
    /// Muir Glacier.
    MuirGlacier,
    /// Arrow Glacier.
    ArrowGlacier,
    /// Gray Glacier.
    GrayGlacier,
    /// Paris.
    Paris,
    /// Shanghai.
    Shanghai,
    /// Ramanujan.
    Ramanujan,
    /// Niels.
    Niels,
    /// MirrorSync.
    MirrorSync,
    /// Bruno.
    Bruno,
    /// Euler.
    Euler,
    /// Nano.
    Nano,
    /// Moran.
    Moran,
    /// Gibbs.
    Gibbs,
    /// Planck.
    Planck,
    /// Luban.
    Luban,
    /// Plato.
    Plato,
    /// Berlin.
    Berlin,
    /// London.
    London,
    /// Hertz
    Hertz,
}

impl Hardfork {
    /// Get the [ForkId] for this hardfork in the given spec, if the fork is activated at any point.
    pub fn fork_id(&self, spec: &ChainSpec) -> Option<ForkId> {
        match spec.fork(*self) {
            ForkCondition::Never => None,
            _ => Some(spec.fork_id(&spec.fork(*self).satisfy())),
        }
    }

    /// Get the [ForkFilter] for this hardfork in the given spec, if the fork is activated at any
    /// point.
    pub fn fork_filter(&self, spec: &ChainSpec) -> Option<super::fork::ForkFilter> {
        match spec.fork(*self) {
            ForkCondition::Never => None,
            _ => Some(spec.fork_filter(spec.fork(*self).satisfy())),
        }
    }
}
