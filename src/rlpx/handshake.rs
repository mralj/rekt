use bytes::{BufMut, BytesMut};
use open_fastrlp::{Encodable, RlpEncodable};
use rand::{thread_rng, Rng};
use secp256k1::SECP256K1;

use crate::types::hash::{H256, H512};

use super::{utils::*, Connection};

const AUT_VERSION: u8 = 4;

impl Connection {
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
            protocol_version: AUT_VERSION,
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
        encrypt_message(
            &self.remote_public_key.unwrap(),
            &unencrypted,
            &mut encrypted,
        );

        let len_bytes = u16::try_from(encrypted.len()).unwrap().to_be_bytes();
        out[..len_bytes.len()].copy_from_slice(&len_bytes);

        out.unsplit(encrypted);
        buf.unsplit(out);
    }
}
