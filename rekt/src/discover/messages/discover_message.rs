use bytes::{BufMut, Bytes, BytesMut};
use ethers::utils::keccak256;
use open_fastrlp::Encodable;
use secp256k1::ecdsa::RecoverableSignature;
use secp256k1::{SecretKey, SECP256K1};

use crate::discover::decoder::MAX_PACKET_SIZE;
use crate::types::hash::H256;

use super::enr::{EnrRequest, EnrResponse};
use super::find_node::FindNode;
use super::ping_pong_messages::{PingMessage, PongMessage};

pub(crate) const DEFAULT_MESSAGE_EXPIRATION: u64 = 20;

pub enum DiscoverMessageType {
    Ping = 1,
    Pong = 2,
    FindNode = 3,
    Neighbors = 4,
    EnrRequest = 5,
    EnrResponse = 6,
}

impl TryFrom<u8> for DiscoverMessageType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DiscoverMessageType::Ping),
            2 => Ok(DiscoverMessageType::Pong),
            3 => Ok(DiscoverMessageType::FindNode),
            4 => Ok(DiscoverMessageType::Neighbors),
            5 => Ok(DiscoverMessageType::EnrRequest),
            6 => Ok(DiscoverMessageType::EnrResponse),
            _ => Err(()),
        }
    }
}

impl DiscoverMessageType {
    pub fn discover_msg_should_be_handled(&self) -> bool {
        match self {
            DiscoverMessageType::Ping => true,
            DiscoverMessageType::EnrRequest => true,
            DiscoverMessageType::EnrResponse => true,
            DiscoverMessageType::Neighbors => true,
            _ => false,
        }
    }
}

pub enum DiscoverMessage {
    Ping(PingMessage),
    Pong(PongMessage),
    EnrRequest(EnrRequest),
    EnrResponse(EnrResponse),
    FindNode(FindNode),
}

impl DiscoverMessage {
    pub(super) fn id(&self) -> u8 {
        match &self {
            DiscoverMessage::Ping(_) => 1,
            DiscoverMessage::Pong(_) => 2,
            DiscoverMessage::FindNode(_) => 3,
            DiscoverMessage::EnrRequest(_) => 5,
            DiscoverMessage::EnrResponse(_) => 6,
        }
    }
}

impl Encodable for DiscoverMessage {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            DiscoverMessage::Ping(msg) => msg.encode(out),
            DiscoverMessage::Pong(msg) => msg.encode(out),
            DiscoverMessage::FindNode(msg) => msg.encode(out),
            DiscoverMessage::EnrRequest(msg) => msg.encode(out),
            DiscoverMessage::EnrResponse(msg) => msg.encode(out),
        }
    }
}

impl DiscoverMessage {
    pub fn create_disc_v4_packet(msg: DiscoverMessage, secret_key: &SecretKey) -> Bytes {
        let mut datagram = BytesMut::with_capacity(MAX_PACKET_SIZE);

        let mut sig_bytes = datagram.split_off(H256::len_bytes());
        let mut payload = sig_bytes.split_off(secp256k1::constants::COMPACT_SIGNATURE_SIZE + 1);
        payload.put_u8(msg.id());
        msg.encode(&mut payload);

        let signature: RecoverableSignature = SECP256K1.sign_ecdsa_recoverable(
            &secp256k1::Message::from_slice(keccak256(&payload).as_ref()).unwrap(),
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
}
