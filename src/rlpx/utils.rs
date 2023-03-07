use digest::Digest;
use hmac::{Hmac, Mac};
use secp256k1::{PublicKey, SecretKey};
use sha2::Sha256;

use crate::types::hash::{H256, H512};

use super::errors::RLPXError;

pub(super) fn ecdh_x(public_key: &PublicKey, secret_key: &SecretKey) -> H256 {
    H256::from_slice(&secp256k1::ecdh::shared_secret_point(public_key, secret_key)[..32])
}

/// Converts a [secp256k1::PublicKey] to a [PeerId] by stripping the
/// SECP256K1_TAG_PUBKEY_UNCOMPRESSED tag and storing the rest of the slice in the [PeerId].
pub(super) fn pk2id(pk: &PublicKey) -> H512 {
    H512::from_slice(&pk.serialize_uncompressed()[1..])
}

pub(super) fn kdf(secret: H256, s1: &[u8], dest: &mut [u8]) {
    // SEC/ISO/Shoup specify counter size SHOULD be equivalent
    // to size of hash output, however, it also notes that
    // the 4 bytes is okay. NIST specifies 4 bytes.
    let mut ctr = 1_u32;
    let mut written = 0_usize;
    while written < dest.len() {
        let mut hasher = Sha256::default();
        let ctrs = [
            (ctr >> 24) as u8,
            (ctr >> 16) as u8,
            (ctr >> 8) as u8,
            ctr as u8,
        ];
        hasher.update(ctrs);
        hasher.update(secret.as_bytes());
        hasher.update(s1);
        let d = hasher.finalize();
        dest[written..(written + 32)].copy_from_slice(&d);
        written += 32;
        ctr += 1;
    }
}

pub(super) fn sha256(data: &[u8]) -> H256 {
    H256::from(Sha256::digest(data).as_ref())
}

/// Produces a HMAC_SHA256 digest of the `input_data` and `auth_data` with the given `key`.
/// This is done by accumulating each slice in `input_data` into the HMAC state, then accumulating
/// the `auth_data` and returning the resulting digest.
pub(super) fn hmac_sha256(key: &[u8], input: &[&[u8]], auth_data: &[u8]) -> H256 {
    let mut hmac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    for input in input {
        hmac.update(input);
    }
    hmac.update(auth_data);
    H256::from_slice(&hmac.finalize().into_bytes())
}

pub(super) fn split_at_mut<T>(
    arr: &mut [T],
    idx: usize,
) -> Result<(&mut [T], &mut [T]), RLPXError> {
    if idx > arr.len() {
        return Err(RLPXError::OutOfBounds {
            idx,
            len: arr.len(),
        });
    }
    Ok(arr.split_at_mut(idx))
}
