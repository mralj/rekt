use std::sync::OnceLock;

use open_fastrlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

const ETH_CAPABILITY: &str = "eth";
pub static OUR_CAPABILITES: OnceLock<Vec<Capability>> = OnceLock::new();

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
