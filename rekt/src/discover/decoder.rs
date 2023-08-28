use std::ops::RangeInclusive;

use bytes::{BufMut, Bytes, BytesMut};
use ethers::utils::keccak256;
use open_fastrlp::{Decodable, Encodable};

use crate::types::hash::H256;
use secp256k1::{ecdsa::RecoverableSignature, SecretKey, SECP256K1};

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

pub fn decode_msg(buf: &[u8], is_test: bool) -> Option<PongMessage> {
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
            Some(pong_msg)
        }
        //5 => println!("ENRRequestPacket"),
        _ => None,
    }
}

pub fn create_disc_v4_packet(pong_msg: PongMessage, secret_key: &SecretKey) -> Bytes {
    // allocate max packet size
    let mut datagram = BytesMut::with_capacity(MAX_PACKET_SIZE);

    // since signature has fixed len, we can split and fill the datagram buffer at fixed
    // positions, this way we can encode the message directly in the datagram buffer
    let mut sig_bytes = datagram.split_off(H256::len_bytes());
    let mut payload = sig_bytes.split_off(secp256k1::constants::COMPACT_SIGNATURE_SIZE + 1);
    payload.put_u8(2);
    pong_msg.encode(&mut payload);

    let signature: RecoverableSignature = SECP256K1.sign_ecdsa_recoverable(
        &secp256k1::Message::from_slice(keccak256(&payload).as_ref())
            .expect("is correct MESSAGE_SIZE; qed"),
        secret_key,
    );

    let (rec, sig) = signature.serialize_compact();
    sig_bytes.extend_from_slice(&sig);
    sig_bytes.put_u8(rec.to_i32() as u8);
    sig_bytes.unsplit(payload);

    let hash = keccak256(&sig_bytes);
    datagram.extend_from_slice(&hash);

    datagram.unsplit(sig_bytes);
    datagram.freeze()
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
