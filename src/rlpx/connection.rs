use aes::Aes256;
use bytes::Bytes;
use ctr::Ctr64BE;
use derive_more::Display;
use secp256k1::{PublicKey, SecretKey, SECP256K1};

use crate::types::hash::H256;

use super::mac::MAC;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum RLPXConnectionState {
    /// The first stage of the RLPX handshake, where each side of the connection sends an AUTH
    /// message containing the ephemeral public key, signature of the public key, nonce, and other
    /// metadata.
    Auth,
    /// The second stage of the RLPX handshake, where each side of the connection sends an ACK
    /// message containing the nonce and other metadata.
    Ack,
    /// All other messages can be split into Header and Body
    Header,
    Body,
}

pub struct Connection {
    pub(super) state: RLPXConnectionState,
    /// https://github.com/ethereum/devp2p/blob/master/rlpx.md#node-identity
    /// Our node's secret key used for signing messages it is unique per server
    /// and per specs it should be persisted between node restarts, but in practice we regenerate it
    pub(super) secret_key: SecretKey,

    /// Our node's public key, by this pk we are identified on the network
    pub(super) public_key: PublicKey,

    ///  As name suggest em_ephemeral key is used for each new "session" between 2 nodes
    /// Ephemeral secret key  is "our" part of the _shared secret_
    pub(super) ephemeral_secret_key: SecretKey,
    /// Ephemeral public key is peer's part of the _shared secret_ it will be received from peer via
    /// ACK msg
    #[allow(dead_code)]
    pub(super) ephemeral_public_key: PublicKey,

    //NOTE: this is option type because we don't have remote_public_key
    // if the peer is dialing us (we are the "server" and they are a "client")
    // ofc. if we are dialing peer, we must know public key (it is part of enode:// spec)
    pub(super) remote_public_key: Option<PublicKey>,

    pub(super) ephemeral_shared_secret: Option<H256>,
    pub(super) remote_ephemeral_public_key: Option<PublicKey>,

    /// Nonce is a random value used for authentication, it is generated once per connection
    pub(super) nonce: H256,
    pub(super) remote_nonce: Option<H256>,

    pub(super) ingress_aes: Option<Ctr64BE<Aes256>>,
    pub(super) egress_aes: Option<Ctr64BE<Aes256>>,
    pub(super) ingress_mac: Option<MAC>,
    pub(super) egress_mac: Option<MAC>,

    pub(super) init_msg: Option<Bytes>,
    pub(super) remote_init_msg: Option<Bytes>,

    pub(super) body_size: Option<usize>,
}

impl Connection {
    pub fn new(secret_key: SecretKey, remote_public_key: PublicKey) -> Self {
        let nonce = H256::random();
        let public_key = PublicKey::from_secret_key(SECP256K1, &secret_key);
        let (ephemeral_secret_key, ephemeral_public_key) =
            secp256k1::generate_keypair(&mut secp256k1::rand::thread_rng());

        Self {
            state: RLPXConnectionState::Auth,
            secret_key,
            public_key,
            ephemeral_secret_key,
            ephemeral_public_key,
            remote_public_key: Some(remote_public_key),
            nonce,
            ephemeral_shared_secret: None,
            remote_ephemeral_public_key: None,
            remote_nonce: None,
            ingress_aes: None,
            egress_aes: None,
            ingress_mac: None,
            egress_mac: None,
            init_msg: None,
            remote_init_msg: None,
            body_size: None,
        }
    }

    pub fn body_len(&self) -> usize {
        let len = self.body_size.unwrap();
        (if len % 16 == 0 {
            len
        } else {
            (len / 16 + 1) * 16
        }) + 16
    }
}
