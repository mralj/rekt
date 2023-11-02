use std::net::SocketAddr;

use chrono::Local;
use enr::Enr;
use ethers::utils::keccak256;
use open_fastrlp::Decodable;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{SecretKey, SECP256K1};

use crate::blockchain::bsc_chain_spec::BSC_MAINNET_FORK_FILTER;
use crate::discover::messages::find_node::Neighbours;
use crate::types::hash::{H256, H512};

use super::discover_node::DiscoverNode;
use super::messages::discover_message::{DiscoverMessage, DiscoverMessageType};
use super::messages::enr::EnrResponse;
use super::messages::ping_pong_messages::{PingMessage, PongMessage};
use super::server::Server;

// The following constants are defined in the "docs"
// https://github.com/ethereum/devp2p/blob/master/discv4.md
const HASH_SIZE: usize = 32;
const SIGNATURE_SIZE: usize = 65;
const TYPE_SIZE: usize = 1;
const HEADER_SIZE: usize = HASH_SIZE + SIGNATURE_SIZE;
// Discovery packets are defined to be no larger than 1280 bytes.
// Packets larger than this size will be cut at the end and treated
// as invalid because their hash won't match.
pub const MAX_PACKET_SIZE: usize = 1280;

pub fn decode_msg_and_create_response(
    server: &Server,
    src: &SocketAddr,
    buf: &[u8],
    enr: &Enr<SecretKey>,
) -> Option<DiscoverMessage> {
    let hash = &buf[..HASH_SIZE];
    let signature = &buf[HASH_SIZE..HEADER_SIZE - 1];
    let msg_type = &buf[HEADER_SIZE];
    let msg_data = &mut &buf[HEADER_SIZE + TYPE_SIZE..];

    let recovery_id = match RecoveryId::from_i32(buf[HEADER_SIZE - 1] as i32) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not parse recovery id: {:?}", e);
            return None;
        }
    };

    let recoverable_sig = match RecoverableSignature::from_compact(signature, recovery_id) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not parse recoverable signature: {:?}", e);
            return None;
        }
    };

    let msg = match secp256k1::Message::from_slice(&keccak256(&buf[97..])) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not parse message: {:?}", e);
            return None;
        }
    };

    let pk = match SECP256K1.recover_ecdsa(&msg, &recoverable_sig) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not recover public key: {:?}", e);
            return None;
        }
    };

    let node_id = H512::from_slice(&pk.serialize_uncompressed()[1..]);

    let msg_type = DiscoverMessageType::try_from(*msg_type).ok()?;
    if !msg_type.discover_msg_should_be_handled() {
        return None;
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    match msg_type {
        DiscoverMessageType::Ping => {
            let ping_msg = PingMessage::decode(msg_data).ok()?;

            //println!("[{}] Ping [{:?}]", now, src);
            match server.nodes.entry(node_id) {
                dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                    entry.get_mut().mark_ping_received();
                }
                dashmap::mapref::entry::Entry::Vacant(entry) => {
                    if let Ok(node) = DiscoverNode::from_ping_msg(&ping_msg, node_id) {
                        entry.insert(node);
                    }
                }
            };

            Some(DiscoverMessage::Pong(PongMessage::new(
                ping_msg,
                H256::from_slice(hash),
            )))
        }
        DiscoverMessageType::Pong => {
            match PongMessage::decode(msg_data) {
                Ok(_) => {
                    println!("[{}] Pong [{:?}]", now, src)
                }
                Err(e) => {
                    println!("Could not decode pong message: {:?}", e);
                    return None;
                }
            }

            server.pending_pings.remove(&node_id);
            let node = &mut server.nodes.get_mut(&node_id)?;
            node.mark_pong_received();

            None
        }
        DiscoverMessageType::EnrRequest => Some(DiscoverMessage::EnrResponse(EnrResponse::new(
            H256::from_slice(hash),
            enr.clone(),
        ))),
        DiscoverMessageType::Neighbors => {
            let neighbours = Neighbours::decode(msg_data).ok()?;
            for n in neighbours.nodes {
                println!("Neighbor: {:?}", n)
            }

            None
        }
        DiscoverMessageType::EnrResponse => {
            let enr_response = EnrResponse::decode(msg_data).ok()?;
            let forks_match = {
                if let Some(fork_id) = enr_response.eth_fork_id() {
                    BSC_MAINNET_FORK_FILTER.validate(fork_id).is_ok()
                } else {
                    false
                }
            };
            println!(
                "[{}] ENR Response message [{:?}]: {:?}, is match: {}",
                now, src, enr_response, forks_match
            );
            None
        }
        _ => None,
    }
}

pub fn packet_size_is_valid(size: usize) -> bool {
    size > HEADER_SIZE && size <= MAX_PACKET_SIZE
}
