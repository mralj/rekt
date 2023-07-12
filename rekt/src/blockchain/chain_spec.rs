use std::collections::BTreeMap;

use crate::types::hash::H256;

use super::fork::{ForkFilter, ForkFilterKey, ForkHash, ForkId};
use super::fork_condition::ForkCondition;
use super::hard_fork::Hardfork;
use super::head::Head;

pub struct ChainSpec {
    /// The chain ID
    pub chain: u8,

    /// The hash of the genesis block.
    ///
    /// This acts as a small cache for known chains. If the chain is known, then the genesis hash
    /// is also known ahead of time, and this will be `Some`.
    pub genesis_hash: H256,

    /// The total difficulty of the genesis block.
    pub td: u64,

    /// The active hard forks and their activation conditions
    pub hardforks: BTreeMap<Hardfork, ForkCondition>,

    // Blockchain head
    pub head: Head,
}

impl ChainSpec {
    /// Returns the forks in this specification and their activation conditions.
    pub fn hardforks(&self) -> &BTreeMap<Hardfork, ForkCondition> {
        &self.hardforks
    }

    /// Get the fork condition for the given fork.
    pub fn fork(&self, fork: Hardfork) -> ForkCondition {
        self.hardforks
            .get(&fork)
            .copied()
            .unwrap_or(ForkCondition::Never)
    }

    /// Get an iterator of all hardforks with their respective activation conditions.
    pub fn forks_iter(&self) -> impl Iterator<Item = (Hardfork, ForkCondition)> + '_ {
        self.hardforks.iter().map(|(f, b)| (*f, *b))
    }

    pub fn fork_filter(&self, head: Head) -> ForkFilter {
        let forks = self.forks_iter().filter_map(|(_, condition)| {
            // We filter out TTD-based forks w/o a pre-known block since those do not show up in the
            // fork filter.
            Some(match condition {
                ForkCondition::Block(block) => ForkFilterKey::Block(block),
                ForkCondition::Timestamp(time) => ForkFilterKey::Time(time),
                ForkCondition::TTD {
                    fork_block: Some(block),
                    ..
                } => ForkFilterKey::Block(block),
                _ => return None,
            })
        });

        ForkFilter::new(head, self.genesis_hash, forks)
    }

    pub fn fork_id(&self, head: &Head) -> ForkId {
        let mut curr_forkhash = ForkHash::from(self.genesis_hash);
        let mut current_applied_value = 0;

        for (_, cond) in self.forks_iter() {
            let value = match cond {
                ForkCondition::Block(block) => block,
                ForkCondition::Timestamp(time) => time,
                ForkCondition::TTD {
                    fork_block: Some(block),
                    ..
                } => block,
                _ => continue,
            };

            if cond.active_at_head(head) {
                if value != current_applied_value {
                    curr_forkhash += value;
                    current_applied_value = value;
                }
            } else {
                return ForkId {
                    hash: curr_forkhash,
                    next: value,
                };
            }
        }
        ForkId {
            hash: curr_forkhash,
            next: 0,
        }
    }
}
