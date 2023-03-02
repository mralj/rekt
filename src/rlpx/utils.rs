use aes::{cipher::StreamCipher, Aes128};
use bytes::BytesMut;
use ctr::Ctr64BE;
use digest::crypto_common::KeyIvInit;
use digest::Digest;
use hmac::{Hmac, Mac};
use secp256k1::{PublicKey, SecretKey, SECP256K1};
use sha2::Sha256;

use crate::types::hash::{H128, H256, H512};

use super::errors::RLPXError;

// NOTE: completely C/P from paradigmxyz/reth
// Code here calculates various "crypto stuff"

/// Encrypts RLPX Handshake messages (AUTH and AKC) using ECIES.
/// https://github.com/ethereum/devp2p/blob/master/rlpx.md#ecies-encryption
pub(super) fn encrypt_message(remote_pk: &PublicKey, data: &[u8], out: &mut BytesMut) {
    out.reserve(secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE + 16 + data.len() + 32);

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    out.extend_from_slice(
        &PublicKey::from_secret_key(SECP256K1, &secret_key).serialize_uncompressed(),
    );

    let x = ecdh_x(remote_pk, &secret_key);
    let mut key = [0u8; 32];
    kdf(x, &[], &mut key);

    let enc_key = H128::from_slice(&key[..16]);
    let mac_key = sha256(&key[16..32]);

    let iv = H128::random();
    let mut encryptor = Ctr64BE::<Aes128>::new(enc_key.as_ref().into(), iv.as_ref().into());

    let mut encrypted = data.to_vec();
    encryptor.apply_keystream(&mut encrypted);

    //TODO: reudce magic numbers is this file
    //Theoretically 😁 we could
    //a) set 16 and 32 to named constants, if we knew what they were
    //b) extract 65 + 16 + data.len() + 32 to a named const since secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE is 6

    let total_size: u16 = u16::try_from(65 + 16 + data.len() + 32).unwrap();

    let tag = hmac_sha256(
        mac_key.as_ref(),
        &[iv.as_bytes(), &encrypted],
        &total_size.to_be_bytes(),
    );

    out.extend_from_slice(iv.as_bytes());
    out.extend_from_slice(&encrypted);
    out.extend_from_slice(tag.as_ref());
}

pub(super) fn decrypt_message<'a>(
    secret_key: &SecretKey,
    data: &'a mut [u8],
) -> Result<&'a mut [u8], RLPXError> {
    let (auth_data, encrypted) = split_at_mut(data, 2)?;
    let (pubkey_bytes, encrypted) = split_at_mut(encrypted, 65)?;
    let public_key = PublicKey::from_slice(pubkey_bytes)?;
    let (data_iv, tag_bytes) = split_at_mut(encrypted, encrypted.len() - 32)?;
    let (iv, encrypted_data) = split_at_mut(data_iv, 16)?;
    let tag = H256::from_slice(tag_bytes);

    let x = ecdh_x(&public_key, secret_key);
    let mut key = [0u8; 32];
    kdf(x, &[], &mut key);
    let enc_key = H128::from_slice(&key[..16]);
    let mac_key = sha256(&key[16..32]);

    let check_tag = hmac_sha256(mac_key.as_ref(), &[iv, encrypted_data], auth_data);
    if check_tag != tag {
        println!(
            "Tag check failed, \n expected: {:?}, \n got:      {:?}",
            tag, check_tag
        );
        return Err(RLPXError::TagCheckDecryptFailed);
    }

    let decrypted_data = encrypted_data;

    let mut decryptor = Ctr64BE::<Aes128>::new(enc_key.as_ref().into(), (*iv).into());
    decryptor.apply_keystream(decrypted_data);

    Ok(decrypted_data)
}

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

fn split_at_mut<T>(arr: &mut [T], idx: usize) -> Result<(&mut [T], &mut [T]), RLPXError> {
    if idx > arr.len() {
        return Err(RLPXError::OutOfBounds {
            idx,
            len: arr.len(),
        });
    }
    Ok(arr.split_at_mut(idx))
}
