use derive_more::Display;
use num_derive::ToPrimitive;
use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ETH_PROTOCOL: &str = "eth";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, ToPrimitive)]
pub enum ProtocolVersion {
    Eth65 = 65,
    Eth66 = 66,
    #[default]
    Eth67 = 67,
    Eth68 = 68,
    Unknown = 0,
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
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth67 as usize),
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth66 as usize),
            Protocol::new(ETH_PROTOCOL.to_string(), ProtocolVersion::Eth65 as usize),
        ]
    }

    pub fn match_protocols(peer_protocols: &mut [Protocol]) -> Option<Protocol> {
        peer_protocols.sort_unstable_by(|fst, snd| snd.version.cmp(&fst.version));
        let mut proto = peer_protocols.first()?;
        if proto.name != ETH_PROTOCOL {
            return None;
        }

        //we don't yet support ETH68 so advance to ETH67/66/65
        if proto.version == 68 {
            proto = peer_protocols.get(1)?;
        }

        let p = match proto.version {
            65 => proto.clone(),
            66 => proto.clone(),
            67 => proto.clone(),
            _ => return None,
        };

        Some(p)
    }
}
