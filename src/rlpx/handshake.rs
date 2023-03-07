use aes::Aes256;
use bytes::{BufMut, Bytes, BytesMut};
use ctr::Ctr64BE;
use digest::{crypto_common::KeyIvInit, Digest};
use open_fastrlp::{Encodable, Rlp, RlpEncodable};
use rand::{thread_rng, Rng};
use secp256k1::SECP256K1;
use sha3::Keccak256;

use crate::types::{
    hash::{H128, H256, H512},
    node_record::id2pk,
};

use super::{
    ecies::{decrypt_message, encrypt_message},
    errors::RLPXError,
    mac::MAC,
    utils::*,
    Connection,
};

const AUT_VERSION: u8 = 4;

impl Connection {
    // C/P from paradigmxyz/reth
    // https://github.com/ethereum/devp2p/blob/master/rlpx.md#initial-handshake
    //
    // auth = auth-size || enc-auth-body
    // auth-size = size of enc-auth-body, encoded as a big-endian 16-bit integer
    // auth-vsn = 4
    // auth-body = [sig, initiator-pubk, initiator-nonce, auth-vsn, ...]
    // enc-auth-body = ecies.encrypt(recipient-pubk, auth-body || auth-padding, auth-size)
    // auth-padding = arbitrary data

    fn create_auth_unencrypted(&self) -> BytesMut {
        // Generate signature
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
        // auth-body = [sig, initiator-pubk, initiator-nonce, auth-vsn, ...]
        S {
            sig_bytes: &sig_bytes,
            id: &id,
            nonce: &self.nonce,
            protocol_version: AUT_VERSION,
        }
        .encode(&mut out);

        // auth-padding = arbitrary data
        // as per EIP-8, we add 100-300 bytes of random data (to distinguish between the "new" -
        // now already used for a while - and the "old" handshake)
        out.resize(out.len() + thread_rng().gen_range(100..=300), 0);
        out
    }

    // This will ECIES encrypt the auth message and write it to the buffer
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

        self.init_msg = Some(Bytes::copy_from_slice(&out));

        buf.unsplit(out);
    }

    /// Parse the incoming `ack` message from the given `data` bytes, which are assumed to be
    /// unencrypted. This parses the remote ephemeral pubkey and nonce from the message, and uses
    /// ECDH to compute the shared secret. The shared secret is the x coordinate of the point
    /// returned by ECDH.
    ///
    /// This sets the `remote_ephemeral_public_key` and `remote_nonce`, and
    /// `ephemeral_shared_secret` fields in the ECIES state.
    fn parse_ack_unencrypted(&mut self, data: &[u8]) -> Result<(), RLPXError> {
        let mut data = Rlp::new(data)?;

        //ack-body = [recipient-ephemeral-pubk, recipient-nonce, ack-vsn, ...]
        self.remote_ephemeral_public_key =
            Some(id2pk(data.get_next()?.ok_or(RLPXError::InvalidAckData)?)?);
        self.remote_nonce = Some(data.get_next()?.ok_or(RLPXError::InvalidAckData)?);

        self.ephemeral_shared_secret = Some(ecdh_x(
            &self.remote_ephemeral_public_key.unwrap(),
            &self.ephemeral_secret_key,
        ));
        Ok(())
    }

    /// Read and verify an ack message from the input data.
    pub fn read_ack(&mut self, data: &mut [u8]) -> Result<(), RLPXError> {
        self.remote_init_msg = Some(Bytes::copy_from_slice(data));
        let unencrypted = decrypt_message(&self.secret_key, data)?;
        self.parse_ack_unencrypted(unencrypted)?;
        self.setup_secrets(false);
        Ok(())
    }

    // Secrets represents the connection secret keys which are negotiated during the handshake.
    // The secrets are used to encrypt and decrypt messages.
    // As well as to confirm the authenticity of the messages using the MAC.

    // On each outgoing message, we/peer calculate egress MAC and append it to the message.
    // On each incoming message, we/peer calculate ingress MAC and compare it to the MAC in the
    // message. (from the point of the sender this was egress MAC)
    fn setup_secrets(&mut self, incoming: bool) {
        let mut hasher = Keccak256::new();
        let (fst_nonce, snd_nonce) = if incoming {
            (self.nonce, self.remote_nonce.unwrap())
        } else {
            (self.remote_nonce.unwrap(), self.nonce)
        };
        hasher.update(fst_nonce);
        hasher.update(snd_nonce);

        let h_nonce = H256::from(hasher.finalize().as_ref());

        let iv = H128::default();
        let shared_secret: H256 = {
            let mut hasher = Keccak256::new();
            hasher.update(self.ephemeral_shared_secret.unwrap().0.as_ref());
            hasher.update(h_nonce.0.as_ref());
            H256::from(hasher.finalize().as_ref())
        };

        let aes_secret: H256 = {
            let mut hasher = Keccak256::new();
            hasher.update(self.ephemeral_shared_secret.unwrap().0.as_ref());
            hasher.update(shared_secret.0.as_ref());
            H256::from(hasher.finalize().as_ref())
        };
        self.ingress_aes = Some(Ctr64BE::<Aes256>::new(
            aes_secret.0.as_ref().into(),
            iv.as_ref().into(),
        ));
        self.egress_aes = Some(Ctr64BE::<Aes256>::new(
            aes_secret.0.as_ref().into(),
            iv.as_ref().into(),
        ));

        let mac_secret: H256 = {
            let mut hasher = Keccak256::new();
            hasher.update(self.ephemeral_shared_secret.unwrap().0.as_ref());
            hasher.update(aes_secret.0.as_ref());
            H256::from(hasher.finalize().as_ref())
        };
        self.ingress_mac = Some(MAC::new(mac_secret));
        self.ingress_mac
            .as_mut()
            .unwrap()
            .update((mac_secret ^ self.nonce).as_ref());
        self.ingress_mac
            .as_mut()
            .unwrap()
            .update(self.remote_init_msg.as_ref().unwrap());
        self.egress_mac = Some(MAC::new(mac_secret));
        self.egress_mac
            .as_mut()
            .unwrap()
            .update((mac_secret ^ self.remote_nonce.unwrap()).as_ref());
        self.egress_mac
            .as_mut()
            .unwrap()
            .update(self.init_msg.as_ref().unwrap());
    }
}
