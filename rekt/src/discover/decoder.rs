use std::net::SocketAddr;

use enr::Enr;
use open_fastrlp::Decodable;
use secp256k1::SecretKey;

use crate::types::hash::H256;

use super::messages::discover_message::{DiscoverMessage, DiscoverMessageType};
use super::messages::enr::EnrResponseMessage;
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
    buf: &[u8],
    enr: &Enr<SecretKey>,
    src: &SocketAddr,
) -> Option<DiscoverMessage> {
    let hash = &buf[..HASH_SIZE];
    let _signature = &buf[HASH_SIZE..HEADER_SIZE];
    let msg_type = &buf[HEADER_SIZE..][0];
    let msg_data = &mut &buf[HEADER_SIZE + TYPE_SIZE..];

    let msg_type = DiscoverMessageType::try_from(*msg_type).ok()?;
    if !msg_type.discover_msg_should_be_handled() {
        return None;
    }

    match msg_type {
        DiscoverMessageType::Ping => {
            println!("Ping message received, from {:?}", src);
            let ping_msg = PingMessage::decode(msg_data).ok()?;
            Some(DiscoverMessage::Pong(PongMessage::new(
                ping_msg,
                H256::from_slice(hash),
            )))
        }
        DiscoverMessageType::EnrRequest => {
            println!("ENR request message received, from {:?}", src);
            Some(DiscoverMessage::EnrResponse(EnrResponseMessage::new(
                H256::from_slice(hash),
                enr.clone(),
            )))
        }
        DiscoverMessageType::Pong => {
            println!("Pong message received, from {:?}", src);
            None
        }
        DiscoverMessageType::Neighbors => {
            println!("Neighbors message received, from {:?}", src);
            None
        }
        _ => {
            println!("Msg of type: {}", msg_type);
            None
        }
    }
}

pub fn packet_size_is_valid(size: usize) -> bool {
    size > HEADER_SIZE && size <= MAX_PACKET_SIZE
}
