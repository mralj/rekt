use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

const ETH_CAPABILITY: &str = "eth";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum CapVersion {
    Eth66 = 66,
    #[default]
    Eth67 = 67,
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

    pub fn get_our_capabilities() -> Vec<Capability> {
        vec![
            Capability::new(ETH_CAPABILITY.to_string(), CapVersion::Eth67 as usize),
            Capability::new(ETH_CAPABILITY.to_string(), CapVersion::Eth66 as usize),
        ]
    }
}
