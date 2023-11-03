use std::net::SocketAddr;

use anyhow::Result;
use ethers::utils::keccak256;
use open_fastrlp::Decodable;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::SECP256K1;

use crate::discover::messages::find_node::Neighbours;
use crate::types::hash::H512;

use super::messages::decoded_discover_message::DecodedDiscoverMessage;
use super::messages::discover_message::{DiscoverMessage, DiscoverMessageType};
use super::messages::enr::{EnrRequest, EnrResponse};
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
    src: SocketAddr,
    buf: &[u8],
) -> Result<DecodedDiscoverMessage> {
    let hash = &buf[..HASH_SIZE];
    let signature = &buf[HASH_SIZE..HEADER_SIZE - 1];
    let msg_type = &buf[HEADER_SIZE];
    let msg_data = &mut &buf[HEADER_SIZE + TYPE_SIZE..];

    let recovery_id = RecoveryId::from_i32(buf[HEADER_SIZE - 1] as i32)?;
    let recoverable_sig = RecoverableSignature::from_compact(signature, recovery_id)?;
    let msg = secp256k1::Message::from_slice(&keccak256(&buf[97..]))?;

    let pk = SECP256K1.recover_ecdsa(&msg, &recoverable_sig)?;
    let node_id = H512::from_slice(&pk.serialize_uncompressed()[1..]);

    let msg_type = DiscoverMessageType::try_from(*msg_type).map_err(|e| anyhow::anyhow!(e))?;
    if !msg_type.discover_msg_should_be_handled() {
        anyhow::bail!("Message type {:?} should not be handled", msg_type);
    }

    match msg_type {
        DiscoverMessageType::Ping => {
            let ping_msg = PingMessage::decode(msg_data)?;
            Ok(DecodedDiscoverMessage::new(
                src,
                DiscoverMessage::Ping(ping_msg),
                node_id,
                hash,
            ))
        }
        DiscoverMessageType::Pong => Ok(DecodedDiscoverMessage::new(
            src,
            DiscoverMessage::Pong(PongMessage::decode(msg_data)?),
            node_id,
            hash,
        )),
        DiscoverMessageType::EnrRequest => Ok(DecodedDiscoverMessage::new(
            src,
            DiscoverMessage::EnrRequest(EnrRequest::new()),
            node_id,
            hash,
        )),
        DiscoverMessageType::Neighbors => Ok(DecodedDiscoverMessage::new(
            src,
            DiscoverMessage::Neighbours(Neighbours::decode(msg_data)?),
            node_id,
            hash,
        )),

        DiscoverMessageType::EnrResponse => Ok(DecodedDiscoverMessage::new(
            src,
            DiscoverMessage::EnrResponse(EnrResponse::decode(msg_data)?),
            node_id,
            hash,
        )),
        _ => anyhow::bail!("Unknown message type"),
    }
}

pub fn packet_size_is_valid(size: usize) -> bool {
    size > HEADER_SIZE && size <= MAX_PACKET_SIZE
}
