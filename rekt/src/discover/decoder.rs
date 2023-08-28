use std::ops::RangeInclusive;

use bytes::Bytes;
use open_fastrlp::Decodable;

use crate::types::hash::H256;

use super::messages::{PingMessage, PongMessage};

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

pub fn packet_size_is_valid(size: usize) -> bool {
    if size <= HEADER_SIZE {
        return false;
    }

    if size > MAX_PACKET_SIZE {
        return false;
    }

    true
}

pub fn decode_msg(buf: &[u8], is_test: bool) -> Option<Bytes> {
    let hash = &buf[..HASH_SIZE];
    let _signature = &buf[HASH_SIZE..HEADER_SIZE];
    let msg_type = &buf[HEADER_SIZE..][0];
    let msg_data = &mut &buf[HEADER_SIZE + TYPE_SIZE..];

    if !msg_type_is_valid(msg_type) {
        return None;
    }

    match msg_type {
        1 => {
            let ping_msg = PingMessage::decode(msg_data);
            if ping_msg.is_err() {
                println!("PingMessage decode error: {:?}", ping_msg);
                return None;
            }

            if is_test {
                println!("PingMessage: {:?}", ping_msg);
            }

            let pong_msg = PongMessage::new(ping_msg.unwrap(), H256::from_slice(hash));
            Some(pong_msg.rlp_encode())
        }
        //5 => println!("ENRRequestPacket"),
        _ => None,
    }
}

fn msg_type_is_valid(msg_type: &u8) -> bool {
    // match msg_type {
    //     1 => println!("Ping"),
    //     2 => println!("Pong"),
    //     3 => println!("Find NodePacket"),
    //     4 => println!("NeighborsPacket"),
    //     5 => println!("ENRRequestPacket"),
    //     6 => println!("ENRResponsePacket"),
    //     _ => println!("Unknown"),
    // }

    (1u8..=6u8).contains(msg_type)
}
