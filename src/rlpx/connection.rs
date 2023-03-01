use secp256k1::{PublicKey, SecretKey, SECP256K1};

use crate::types::hash::H256;

pub struct Connection {
    pub(super) secret_key: SecretKey,
    pub(super) public_key: PublicKey,

    pub(super) ephemeral_secret_key: SecretKey,
    pub(super) ephemeral_public_key: PublicKey,

    pub(super) remote_public_key: Option<PublicKey>,

    pub(super) nonce: H256,
}

impl Connection {
    pub fn new(secret_key: SecretKey, remote_public_key: PublicKey) -> Self {
        let nonce = H256::random();
        let public_key = PublicKey::from_secret_key(SECP256K1, &secret_key);
        let ephemeral_secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let ephemeral_public_key = PublicKey::from_secret_key(SECP256K1, &ephemeral_secret_key);

        Self {
            secret_key,
            public_key,
            ephemeral_secret_key,
            ephemeral_public_key,
            remote_public_key: Some(remote_public_key),
            nonce,
        }
    }
}
