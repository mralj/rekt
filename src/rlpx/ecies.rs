use aes::Aes128;
use bytes::BytesMut;
use cipher::StreamCipher;
use ctr::Ctr64BE;
use digest::crypto_common::KeyIvInit;
use secp256k1::{constants::UNCOMPRESSED_PUBLIC_KEY_SIZE, PublicKey, SecretKey, SECP256K1};

use crate::types::hash::{H128, H256};

use super::{errors::RLPXError, utils::*};

// NOTE: completely C/P from paradigmxyz/reth, which is C/P from this project:
// https://github.com/vorot93/devp2p

//This is a random initialization vector that was generated for the symmetric encryption algorithm.
//It is encrypted using the recipient's public key and an asymmetric encryption algorithm such as RSA.
const ECIES_IV_SIZE: usize = 16;
//This is additional metadata that may be included in the metadata block,
//such as the public key used for encryption or other security-related information.
// DEVP2P uses this "metadata" to store public key
const ECIES_OPTIONAL_METADATA_SIZE: usize = 32;
// DEVP2P has constant ECIES overeat of 113 bytes
const ECIES_METADATA_OVERHEAD: usize =
    ECIES_OPTIONAL_METADATA_SIZE + ECIES_IV_SIZE + UNCOMPRESSED_PUBLIC_KEY_SIZE;

//Per RLPX specs:
//auth = auth-size || enc-auth-body
//auth-size = size of enc-auth-body, encoded as a big-endian 16-bit integer, 16-bit is ofc. 2 bytes
pub(crate) const RLPX_AUTH_MSG_LEN_MARKER: usize = 2;

/// Encrypts RLPX Handshake messages (AUTH and AKC) using ECIES.
/// https://github.com/ethereum/devp2p/blob/master/rlpx.md#ecies-encryption
pub(super) fn encrypt_message(remote_pk: &PublicKey, data: &[u8], out: &mut BytesMut) {
    out.reserve(data.len() + ECIES_METADATA_OVERHEAD);

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    out.extend_from_slice(
        &PublicKey::from_secret_key(SECP256K1, &secret_key).serialize_uncompressed(),
    );

    let x = ecdh_x(remote_pk, &secret_key);
    let mut key = [0u8; ECIES_OPTIONAL_METADATA_SIZE];
    kdf(x, &[], &mut key);

    let enc_key = H128::from_slice(&key[..ECIES_IV_SIZE]);
    let mac_key = sha256(&key[ECIES_IV_SIZE..ECIES_OPTIONAL_METADATA_SIZE]);

    let iv = H128::random();
    let mut encryptor = Ctr64BE::<Aes128>::new(enc_key.as_ref().into(), iv.as_ref().into());

    let mut encrypted = data.to_vec();
    encryptor.apply_keystream(&mut encrypted);

    let total_size: u16 = u16::try_from(ECIES_METADATA_OVERHEAD + data.len()).unwrap();

    let tag = hmac_sha256(
        mac_key.as_ref(),
        &[iv.as_bytes(), &encrypted],
        &total_size.to_be_bytes(),
    );

    out.extend_from_slice(iv.as_bytes());
    out.extend_from_slice(&encrypted);
    out.extend_from_slice(tag.as_ref());
}

/// Decrypts RLPX Handshake messages (AUTH and AKC) using ECIES.
/// https://github.com/ethereum/devp2p/blob/master/rlpx.md#ecies-encryption
pub(super) fn decrypt_message<'a>(
    secret_key: &SecretKey,
    data: &'a mut [u8],
) -> Result<&'a mut [u8], RLPXError> {
    let (auth_data, encrypted) = split_at_mut(data, RLPX_AUTH_MSG_LEN_MARKER)?;
    let (pubkey_bytes, encrypted) = split_at_mut(encrypted, UNCOMPRESSED_PUBLIC_KEY_SIZE)?;
    let public_key = PublicKey::from_slice(pubkey_bytes)?;
    let (data_iv, tag_bytes) =
        split_at_mut(encrypted, encrypted.len() - ECIES_OPTIONAL_METADATA_SIZE)?;
    let (iv, encrypted_data) = split_at_mut(data_iv, ECIES_IV_SIZE)?;
    let tag = H256::from_slice(tag_bytes);

    let x = ecdh_x(&public_key, secret_key);
    let mut key = [0u8; ECIES_OPTIONAL_METADATA_SIZE];
    kdf(x, &[], &mut key);
    let enc_key = H128::from_slice(&key[..ECIES_IV_SIZE]);
    let mac_key = sha256(&key[ECIES_IV_SIZE..ECIES_OPTIONAL_METADATA_SIZE]);

    let check_tag = hmac_sha256(mac_key.as_ref(), &[iv, encrypted_data], auth_data);
    if check_tag != tag {
        return Err(RLPXError::TagCheckDecryptFailed);
    }

    let decrypted_data = encrypted_data;

    let mut decryptor = Ctr64BE::<Aes128>::new(enc_key.as_ref().into(), (*iv).into());
    decryptor.apply_keystream(decrypted_data);

    Ok(decrypted_data)
}
