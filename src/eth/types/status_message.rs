use std::fmt::{Debug, Display};

use bytes::BytesMut;
use once_cell::sync::Lazy;
use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::blockchain::bsc_chain_spec::BSC_MAINNET_FORK_ID;
use crate::blockchain::fork::ForkId;
use crate::blockchain::BSC_MAINNET;
use crate::types::hash::H256;

pub static OUR_ETH_STATUS_MSG: Lazy<Status> = Lazy::new(Status::default);

#[derive(Copy, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Status {
    /// The current protocol version. For example, peers running `eth/66` would have a version of
    /// 66.
    pub version: u8,

    /// The chain id, as introduced in
    /// [EIP155](https://eips.ethereum.org/EIPS/eip-155#list-of-chain-ids).
    pub chain: u64,

    /// Total difficulty of the best chain.
    pub total_difficulty: u64,

    /// The highest difficulty block hash the peer has seen
    pub blockhash: H256,

    /// The genesis hash of the peer's chain.
    pub genesis: H256,

    /// The fork identifier, a [CRC32
    /// checksum](https://en.wikipedia.org/wiki/Cyclic_redundancy_check#CRC-32_algorithm) for
    /// identifying the peer's fork as defined by
    /// [EIP-2124](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-2124.md).
    /// This was added in [`eth/64`](https://eips.ethereum.org/EIPS/eip-2364)
    pub forkid: ForkId,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            version: 67,
            chain: BSC_MAINNET.chain as u64,
            total_difficulty: BSC_MAINNET.td,
            blockhash: BSC_MAINNET.genesis_hash,
            genesis: BSC_MAINNET.genesis_hash,
            forkid: *BSC_MAINNET_FORK_ID,
  
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hexed_blockhash = hex::encode(self.blockhash);
        let hexed_genesis = hex::encode(self.genesis);
        write!(
            f,
            "Status {{ version: {}, chain: {}, total_difficulty: {}, blockhash: {}, genesis: {}, forkid: {:X?} }}",
            self.version,
            self.chain,
            self.total_difficulty,
            hexed_blockhash,
            hexed_genesis,
            self.forkid
        )
    }
}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hexed_blockhash = hex::encode(self.blockhash);
        let hexed_genesis = hex::encode(self.genesis);
        if f.alternate() {
            write!(
                f,
                "Status {{\n\tversion: {:?},\n\tchain: {:?},\n\ttotal_difficulty: {:?},\n\tblockhash: {},\n\tgenesis: {},\n\tforkid: {:X?}\n}}",
                self.version,
                self.chain,
                self.total_difficulty,
                hexed_blockhash,
                hexed_genesis,
                self.forkid
            )
        } else {
            write!(
                f,
                "Status {{ version: {:?}, chain: {:?}, total_difficulty: {:?}, blockhash: {}, genesis: {}, forkid: {:X?} }}",
                self.version,
                self.chain,
                self.total_difficulty,
                hexed_blockhash,
                hexed_genesis,
                self.forkid
            )
        }
    }
}

impl Status {
    pub fn rlp_encode(&self) -> BytesMut {
        let mut status_rlp = BytesMut::new();
        16_u8.encode(&mut status_rlp);
        self.encode(&mut status_rlp);
        status_rlp
    }
}
