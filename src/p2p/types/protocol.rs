use std::sync::OnceLock;

use derive_more::Display;
use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ETH_PROTOCOL: &str = "eth";
pub static OUR_PROTOCOLS: OnceLock<Vec<Protocol>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Default, Display)]
pub enum ProtocolVersion {
    Eth66 = 66,
    #[default]
    Eth67 = 67,
}

#[derive(Error, Debug)]
pub enum ProtocolVersionError {
    #[error(
        "Protocol
version {0} is not supported"
    )]
    UnsupportedVersion(usize),
}

impl TryFrom<usize> for ProtocolVersion {
    type Error = ProtocolVersionError;
    fn try_from(version: usize) -> Result<Self, ProtocolVersionError> {
        match version {
            66 => Ok(Self::Eth66),
            67 => Ok(Self::Eth67),
            _ => Err(ProtocolVersionError::UnsupportedVersion(version)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Protocol {
    pub name: String,
    pub version: usize,
}

impl Protocol {
    pub fn new(name: String, version: usize) -> Self {
        Self { name, version }
    }

    pub fn get_our_protocols() -> &'static Vec<Protocol> {
        OUR_PROTOCOLS.get_or_init(|| {
            vec![
                Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth67 as usize),
                Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth66 as usize),
            ]
        })
    }

    pub fn match_protocols(
        peer_protocols: &[Protocol],
        our_protocols: &[Protocol],
    ) -> Option<Protocol> {
        let mut eth_protocols: Vec<Protocol> = peer_protocols
            .iter()
            .cloned()
            .filter(|p| p.name == ETH_PROTOCOL)
            .collect();

        if eth_protocols.is_empty() {
            return None;
        }

        eth_protocols.sort_unstable_by(|fst, snd| snd.version.cmp(&fst.version));
        eth_protocols
            .into_iter()
            .find(|p| our_protocols.contains(p))
    }
}
