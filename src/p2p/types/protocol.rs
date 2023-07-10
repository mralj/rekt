use derive_more::Display;
use num_derive::ToPrimitive;
use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ETH_PROTOCOL: &str = "eth";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, ToPrimitive)]
pub enum ProtocolVersion {
    Eth66 = 66,
    #[default]
    Eth67 = 67,
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
            66 => Self::Eth66,
            _ => Self::Eth67,
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
        ]
    }

    pub fn match_protocols(peer_protocols: &mut [Protocol]) -> Option<Protocol> {
        peer_protocols.sort_unstable_by(|fst, snd| snd.version.cmp(&fst.version));
        let proto = peer_protocols.first();

        match proto {
            Some(p) if p.version == 67 && p.name == ETH_PROTOCOL => proto.cloned(),
            Some(p) if p.version == 66 && p.name == ETH_PROTOCOL => proto.cloned(),
            _ => None,
        }
    }
}
