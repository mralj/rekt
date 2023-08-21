use std::fmt::{Debug, Display};

use bytes::BytesMut;
use derive_more::Display;
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::blockchain::bsc_chain_spec::{BSC_MAINNET_FORK_FILTER, BSC_MAINNET_FORK_ID};
use crate::blockchain::fork::ForkId;
use crate::blockchain::BSC_MAINNET;
use crate::eth::types::protocol::EthProtocol;
use crate::p2p::protocol::ProtocolVersion;
use crate::types::hash::H256;

use super::eth_message::EthMessage;

static OUR_STATUS_MESSAGE_ETH_66: OnceCell<EthMessage> = OnceCell::new();
static OUR_STATUS_MESSAGE_ETH_67: OnceCell<EthMessage> = OnceCell::new();
static OUR_UPGRADE_STATUS_MESSAGE: OnceCell<EthMessage> = OnceCell::new();

#[derive(Copy, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct StatusMessage {
    /// The current protocol version. For example, peers running `eth/66` would have a version of
    /// 66.
    pub version: u8,

    /// The chain id, as introduced in
    /// [EIP155](https://eips.ethereum.org/EIPS/eip-155#list-of-chain-ids).
    pub chain: u8,

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

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            version: ProtocolVersion::default() as u8,
            // negotiated
            chain: BSC_MAINNET.chain,
            total_difficulty: BSC_MAINNET.td,
            blockhash: BSC_MAINNET.genesis_hash,
            genesis: BSC_MAINNET.genesis_hash,
            forkid: *BSC_MAINNET_FORK_ID,
        }
    }
}

impl Display for StatusMessage {
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

impl Debug for StatusMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hexed_blockhash = hex::encode(self.blockhash);
        let hexed_genesis = hex::encode(self.genesis);
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
    }
}

impl StatusMessage {
    pub fn make_our_status_msg(proto_v_negotiated: &ProtocolVersion) -> Self {
        Self {
            version: proto_v_negotiated.to_u8().unwrap(),
            ..Self::default()
        }
    }

    pub fn get(proto_v_negotiated: &ProtocolVersion) -> EthMessage {
        match proto_v_negotiated {
            ProtocolVersion::Eth66 => OUR_STATUS_MESSAGE_ETH_66.get_or_init(|| {
                let status = Self {
                    version: 66,
                    ..Self::default()
                };
                let mut status_rlp = BytesMut::new();
                status.encode(&mut status_rlp);
                EthMessage::new(EthProtocol::StatusMsg, status_rlp)
            }),
            ProtocolVersion::Eth67 => OUR_STATUS_MESSAGE_ETH_67.get_or_init(|| {
                let status = Self {
                    version: 67,
                    ..Self::default()
                };
                let mut status_rlp = BytesMut::new();
                status.encode(&mut status_rlp);
                EthMessage::new(EthProtocol::StatusMsg, status_rlp)
            }),
        }
        .clone()
    }

    pub fn rlp_encode(&self) -> BytesMut {
        let mut status_rlp = BytesMut::new();
        self.encode(&mut status_rlp);
        status_rlp
    }

    pub fn validate(
        peer_status_msg: &StatusMessage,
        proto_v_negotiated: &ProtocolVersion,
    ) -> Result<(), &'static str> {
        if *proto_v_negotiated as u8 != peer_status_msg.version {
            error!(
                "Protocol version mismatch, received {:?}",
                peer_status_msg.version
            );
            return Err("Protocol version mismatch");
        }

        if BSC_MAINNET.chain != peer_status_msg.chain {
            error!("Chain ID mismatch, received {:?}", peer_status_msg.chain);
            return Err("Chain ID mismatch");
        }

        if BSC_MAINNET.genesis_hash != peer_status_msg.genesis {
            error!(
                "Genesis hash mismatch, received {:?}",
                peer_status_msg.genesis
            );
            return Err("Genesis hash mismatch");
        }

        if BSC_MAINNET_FORK_FILTER
            .validate(peer_status_msg.forkid)
            .is_err()
        {
            error!("Fork ID mismatch, received {:X?}", peer_status_msg.forkid);
            return Err("Fork ID Mismatch");
        }

        Ok(())
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    PartialEq,
    Eq,
    RlpEncodable,
    RlpDecodable,
    Serialize,
    Deserialize,
    Default,
)]
struct UpgradeStatusExtension {
    disable_peer_tx_broadcast: bool,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    PartialEq,
    Eq,
    RlpEncodable,
    RlpDecodable,
    Serialize,
    Deserialize,
    Default,
)]
pub struct UpgradeStatusMessage {
    extension: UpgradeStatusExtension,
}

impl UpgradeStatusMessage {
    pub fn get() -> EthMessage {
        OUR_UPGRADE_STATUS_MESSAGE
            .get_or_init(|| {
                EthMessage::new(
                    EthProtocol::UpgradeStatusMsg,
                    UpgradeStatusMessage::default().rlp_encode(),
                )
            })
            .clone()
    }

    pub fn rlp_encode(&self) -> BytesMut {
        let mut status_rlp = BytesMut::new();
        self.encode(&mut status_rlp);
        status_rlp
    }
}
