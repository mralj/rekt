use derive_more::Display;
use num_derive::ToPrimitive;
use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ETH_PROTOCOL: &str = "eth";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, ToPrimitive, PartialOrd)]
pub enum ProtocolVersion {
    Unknown = 0,
    Eth65 = 65,
    Eth66 = 66,
    #[default]
    Eth67 = 67,
    Eth68 = 68,
}

#[derive(Error, Debug, Copy, Clone)]
pub enum ProtocolVersionError {
    #[error(
        "Protocol
version {0} is not supported"
    )]
    UnsupportedVersion(usize),
}

impl From<usize> for ProtocolVersion {
    fn from(version: usize) -> Self {
        match version {
            65 => Self::Eth65,
            66 => Self::Eth66,
            67 => Self::Eth67,
            68 => Self::Eth68,
            _ => Self::Unknown,
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

    pub fn get_our_protocols() -> Vec<Protocol> {
        vec![
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth68 as usize),
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth67 as usize),
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth66 as usize),
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth65 as usize),
        ]
    }

    pub fn match_protocols(peer_protocols: &mut [Protocol]) -> Option<Protocol> {
        peer_protocols.sort_unstable_by(|fst, snd| snd.version.cmp(&fst.version));
        let proto = peer_protocols.first()?;
        if proto.name != ETH_PROTOCOL {
            return None;
        }

        let p = match proto.version {
            68 => proto.clone(),
            67 => proto.clone(),
            66 => proto.clone(),
            65 => proto.clone(),
            _ => return None,
        };

        Some(p)
    }
}
