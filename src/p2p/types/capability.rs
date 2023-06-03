use std::sync::OnceLock;

use derive_more::Display;
use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ETH_CAPABILITY: &str = "eth";
pub static OUR_CAPABILITES: OnceLock<Vec<Capability>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Default, Display)]
pub enum CapVersion {
    Eth66 = 66,
    #[default]
    Eth67 = 67,
}

#[derive(Error, Debug)]
pub enum CapVersionError {
    #[error("Capability version {0} is not supported")]
    UnsupportedVersion(usize),
}

impl TryFrom<usize> for CapVersion {
    type Error = CapVersionError;
    fn try_from(version: usize) -> Result<Self, CapVersionError> {
        match version {
            66 => Ok(Self::Eth66),
            67 => Ok(Self::Eth67),
            _ => Err(CapVersionError::UnsupportedVersion(version)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub version: usize,
}

impl Capability {
    pub fn new(name: String, version: usize) -> Self {
        Self { name, version }
    }

    pub fn get_our_capabilities() -> &'static Vec<Capability> {
        OUR_CAPABILITES.get_or_init(|| {
            vec![
                Capability::new(ETH_CAPABILITY.to_string(), CapVersion::Eth67 as usize),
                Capability::new(ETH_CAPABILITY.to_string(), CapVersion::Eth66 as usize),
            ]
        })
    }

    pub fn match_capabilities<'c>(
        peer_caps: &'c [Capability],
        our_caps: &[Capability],
    ) -> Option<&'c Capability> {
        let mut eth_caps: Vec<&Capability> = peer_caps
            .iter()
            .filter(|c| c.name == ETH_CAPABILITY)
            .collect();

        if eth_caps.is_empty() {
            return None;
        }

        eth_caps.sort_unstable_by(|fst, snd| snd.version.cmp(&fst.version));
        eth_caps.iter().find(|cap| our_caps.contains(*cap)).copied()
    }
}
