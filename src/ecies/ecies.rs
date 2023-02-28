use aes::{cipher::StreamCipher, Aes128};
use bytes::{BufMut, BytesMut};
use ctr::Ctr64BE;
use digest::crypto_common::KeyIvInit;
use open_fastrlp::{Encodable, RlpEncodable};
use rand::{thread_rng, Rng};
use secp256k1::{PublicKey, SecretKey, SECP256K1};

use crate::{
    ecies::utils::{ecdh_x, pk2id},
    types::hash::{H128, H256, H512},
};

use super::utils::{hmac_sha256, kdf, sha256};

pub const PROTOCOL_VERSION: usize = 4;

pub struct ECIES {
    secret_key: SecretKey,
    public_key: PublicKey,

    ephemeral_secret_key: SecretKey,
    ephemeral_public_key: PublicKey,

    remote_public_key: Option<PublicKey>,

    nonce: H256,
}

impl ECIES {
    fn create_auth_unencrypted(&self) -> BytesMut {
        let x = ecdh_x(&self.remote_public_key.unwrap(), &self.secret_key);
        let msg = x ^ self.nonce;
        let (rec_id, sig) = SECP256K1
            .sign_ecdsa_recoverable(
                &secp256k1::Message::from_slice(msg.as_bytes()).unwrap(),
                &self.ephemeral_secret_key,
            )
            .serialize_compact();

        let mut sig_bytes = [0u8; 65];
        sig_bytes[..64].copy_from_slice(&sig);
        sig_bytes[64] = rec_id.to_i32() as u8;

        let id = pk2id(&self.public_key);

        #[derive(RlpEncodable)]
        struct S<'a> {
            sig_bytes: &'a [u8; 65],
            id: &'a H512,
            nonce: &'a H256,
            protocol_version: u8,
        }

        let mut out = BytesMut::new();
        S {
            sig_bytes: &sig_bytes,
            id: &id,
            nonce: &self.nonce,
            protocol_version: PROTOCOL_VERSION as u8,
        }
        .encode(&mut out);

        out.resize(out.len() + thread_rng().gen_range(100..=300), 0);
        out
    }

    pub fn write_auth(&mut self, buf: &mut BytesMut) {
        let unencrypted = self.create_auth_unencrypted();

        let mut out = buf.split_off(buf.len());
        out.put_u16(0);

        let mut encrypted = out.split_off(out.len());
        self.encrypt_message(&unencrypted, &mut encrypted);

        let len_bytes = u16::try_from(encrypted.len()).unwrap().to_be_bytes();
        out[..len_bytes.len()].copy_from_slice(&len_bytes);

        out.unsplit(encrypted);

        // self.init_msg = Some(Bytes::copy_from_slice(&out));

        buf.unsplit(out);
    }

    fn encrypt_message(&self, data: &[u8], out: &mut BytesMut) {
        out.reserve(secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE + 16 + data.len() + 32);

        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        out.extend_from_slice(
            &PublicKey::from_secret_key(SECP256K1, &secret_key).serialize_uncompressed(),
        );

        let x = ecdh_x(&self.remote_public_key.unwrap(), &secret_key);
        let mut key = [0u8; 32];
        kdf(x, &[], &mut key);

        let enc_key = H128::from_slice(&key[..16]);
        let mac_key = sha256(&key[16..32]);

        let iv = H128::random();
        let mut encryptor = Ctr64BE::<Aes128>::new(enc_key.as_ref().into(), iv.as_ref().into());

        let mut encrypted = data.to_vec();
        encryptor.apply_keystream(&mut encrypted);

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
}
