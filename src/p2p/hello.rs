use bytes::BytesMut;
use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::types::hash::H512;

use super::types::Capability;

const DEFAULT_P2P_PROTOCOL_VERSION: usize = 5;
const DEFAULT_PORT: usize = 30311;

/// Message used in the `p2p` handshake, containing information about the supported RLPx protocol
/// version and capabilities.
#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct HelloMessage {
    /// The version of the `p2p` protocol.
    pub protocol_version: usize,
    /// Specifies the client software identity, as a human-readable string (e.g.
    /// "Ethereum(++)/1.0.0").
    pub client_version: String,
    /// The list of supported capabilities and their versions.
    pub capabilities: Vec<Capability>,
    /// The port that the client is listening on, zero indicates the client is not listening.
    //TODO: at the time of writing (we are only able to connect to "static nodes")
    // for all the nodes this field is 0. I would think this is bug,
    // But! GETH node also prints for the majority of nodes this field as 0
    // What's more, in the GETH code I don't see that when we are sending hello message this field
    // is even set
    pub port: usize,
    /// The secp256k1 public key corresponding to the node's private key.
    pub id: H512,
}

impl Default for HelloMessage {
    fn default() -> Self {
        Self {
            protocol_version: DEFAULT_P2P_PROTOCOL_VERSION,
            port: DEFAULT_PORT,
            capabilities: Capability::get_our_capabilities(),
            // we could write "anything" here, like "madnode", but we don't want to bring attention
            // to us, thus empty string. Alternatively we could lie that we are GETH node
            client_version: String::new(),
            id: H512::default(),
        }
    }
}

impl HelloMessage {
    pub fn make_our_hello_message(id: H512) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    pub fn rlp_encode(&self) -> BytesMut {
        let mut hello_rlp = BytesMut::new();
        self.encode(&mut hello_rlp);
        hello_rlp
    }
}
