use std::net::SocketAddr;

use chrono::Local;
use enr::Enr;
use open_fastrlp::Decodable;
use secp256k1::SecretKey;

use crate::blockchain::bsc_chain_spec::BSC_MAINNET_FORK_FILTER;
use crate::discover::messages::find_node::Neighbours;
use crate::types::hash::H256;

use super::messages::discover_message::{DiscoverMessage, DiscoverMessageType};
use super::messages::enr::EnrResponse;
use super::messages::ping_pong_messages::{PingMessage, PongMessage};

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
    src: &SocketAddr,
    buf: &[u8],
    enr: &Enr<SecretKey>,
) -> Option<DiscoverMessage> {
    let hash = &buf[..HASH_SIZE];
    let _signature = &buf[HASH_SIZE..HEADER_SIZE];
    let msg_type = &buf[HEADER_SIZE..][0];
    let msg_data = &mut &buf[HEADER_SIZE + TYPE_SIZE..];

    let msg_type = DiscoverMessageType::try_from(*msg_type).ok()?;
    if !msg_type.discover_msg_should_be_handled() {
        return None;
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    match msg_type {
        DiscoverMessageType::Ping => {
            println!("[{}] Ping message [{:?}]", now, src);
            let ping_msg = PingMessage::decode(msg_data).ok()?;
            Some(DiscoverMessage::Pong(PongMessage::new(
                ping_msg,
                H256::from_slice(hash),
            )))
        }
        DiscoverMessageType::EnrRequest => {
            println!("[{}] ENR message [{:?}]", now, src);
            Some(DiscoverMessage::EnrResponse(EnrResponse::new(
                H256::from_slice(hash),
                enr.clone(),
            )))
        }
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
