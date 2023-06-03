use std::fmt::Display;

use bytes::BytesMut;
use open_fastrlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::{p2p::types::Protocol, types::hash::H512};

const DEFAULT_P2P_PROTOCOL_VERSION: usize = 5;
const DEFAULT_PORT: usize = 30311;

/// Message used in the `p2p` handshake, containing information about the supported RLPx protocol
/// version and protocols.
#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct HelloMessage {
    /// The version of the `p2p` protocol.
    pub protocol_version: usize,
    /// Specifies the client software identity, as a human-readable string (e.g.
    /// "Ethereum(++)/1.0.0").
    pub client_version: String,
    /// The list of supported protocols and their versions.
    pub protocols: Vec<Protocol>,
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
            protocols: Protocol::get_our_protocols().clone(),
            // we could write "anything" here, like "madnode", but we don't want to bring attention
            // to us, thus empty string. Alternatively we could lie that we are GETH node
            client_version: String::new(),
            id: H512::default(),
        }
    }
}

impl Display for HelloMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Protocol Version: {}\nClient Version: {}\nProtocols: {:?}\nPort: {}\nID: {}",
            self.protocol_version, self.client_version, self.protocols, self.port, self.id
        )
    }
}

impl HelloMessage {
    pub fn empty() -> Self {
        Self {
            protocol_version: DEFAULT_P2P_PROTOCOL_VERSION,
            client_version: String::new(),
            protocols: Vec::new(),
            port: 0,
            id: H512::zero(),
        }
    }
    pub fn make_our_hello_message(id: H512) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    pub fn rlp_encode(&self) -> BytesMut {
        let mut hello_rlp = BytesMut::new();
        super::P2PMessageID::Hello.encode(&mut hello_rlp);
        self.encode(&mut hello_rlp);
        hello_rlp
    }
}
