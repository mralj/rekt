use secp256k1::PublicKey;
use std::{
    net::{IpAddr, Ipv4Addr},
    num::ParseIntError,
    str::FromStr,
};
use url::{Host, Url};

use super::hash::H512;

const SIZE_OF_PUBLIC_KEY: usize = 64;
const SIZE_OF_PUBLIC_KEY_WITH_REC_ID: usize = SIZE_OF_PUBLIC_KEY + 1;
// SECP256K1_TAG_PUBKEY_UNCOMPRESSED = 0x04
// see: https://github.com/bitcoin-core/secp256k1/blob/master/include/secp256k1.h#L211
const SECP256K1_TAG_PUBKEY_UNCOMPRESSED: u8 = 0x04;

#[derive(Debug, thiserror::Error)]
pub enum NodeRecordParseError {
    #[error("Failed to parse url: {0}")]
    InvalidUrl(String),
    #[error("Failed to parse id")]
    InvalidId(String),
    #[error("Failed to discport query: {0}")]
    Discport(ParseIntError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRecord {
    /// The Address of a node.
    pub address: IpAddr,
    /// TCP port of the port that accepts connections.
    pub tcp_port: u16,
    /// UDP discovery port.
    pub udp_port: u16,
    /// Public id of the discovery service
    pub id: H512,
    /// Public key of a node
    pub pub_key: PublicKey,
    ///string representation of the node record
    pub str: String,
}

impl NodeRecord {
    pub fn get_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.address, self.tcp_port)
    }
}

impl FromStr for NodeRecord {
    type Err = NodeRecordParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s).map_err(|e| NodeRecordParseError::InvalidUrl(e.to_string()))?;

        let address = match url.host() {
            Some(Host::Ipv4(ip)) => IpAddr::V4(ip),
            Some(Host::Ipv6(ip)) => IpAddr::V6(ip),
            Some(Host::Domain(ip)) => IpAddr::V4(
                Ipv4Addr::from_str(ip)
                    .map_err(|e| NodeRecordParseError::InvalidUrl(e.to_string()))?,
            ),
            _ => {
                return Err(NodeRecordParseError::InvalidUrl(format!(
                    "invalid host: {url:?}"
                )))
            }
        };
        let port = url
            .port()
            .ok_or_else(|| NodeRecordParseError::InvalidUrl("no port specified".to_string()))?;

        let udp_port = match url
            .query_pairs()
            .find(|(maybe_disc, _)| maybe_disc.as_ref() == "discport")
        {
            Some((_, discovery_port)) => discovery_port
                .parse::<u16>()
                .map_err(NodeRecordParseError::Discport)?,
            None => port,
        };

        let id = url
            .username()
            .parse::<H512>()
            .map_err(|e| NodeRecordParseError::InvalidId(e.to_string()))?;

        Ok(Self {
            address,
            id,
            tcp_port: port,
            udp_port,
            pub_key: id2pk(id).map_err(|e| NodeRecordParseError::InvalidId(e.to_string()))?,
            str: s.to_string(),
        })
    }
}

pub fn id2pk(id: H512) -> Result<PublicKey, secp256k1::Error> {
    // NOTE: H512 is used as a PeerId not because it represents a hash, but because 512 bits is
    // enough to represent an uncompressed public key.
    let mut s = [0u8; SIZE_OF_PUBLIC_KEY_WITH_REC_ID];
    s[0] = SECP256K1_TAG_PUBKEY_UNCOMPRESSED;
    s[1..].copy_from_slice(id.as_bytes());
    PublicKey::from_slice(&s)
}

#[cfg(test)]
mod test {
    use std::net::IpAddr;

    use super::NodeRecord;

    #[test]
    fn test_url_parse() {
        let url = "enode://6f8a80d14311c39f35f516fa664deaaaa13e85b2f7493f37f6144d86991ec012937307647bd3b9a82abe2974e1407241d54947bbb39763a4cac9f77166ad92a0@10.3.58.6:30303";
        let node: NodeRecord = url.parse().unwrap();
        let pk = node.pub_key;

        assert_eq!(node, NodeRecord {
            address: IpAddr::V4([10,3,58,6].into()),
            tcp_port: 30303,
            udp_port: 30303,
            id: "6f8a80d14311c39f35f516fa664deaaaa13e85b2f7493f37f6144d86991ec012937307647bd3b9a82abe2974e1407241d54947bbb39763a4cac9f77166ad92a0".parse().unwrap(),
            pub_key: pk,
            str: url.to_string(),
        
        })
    }
    #[test]
    fn test_url_parse_with_disc_port() {
        let url = "enode://6f8a80d14311c39f35f516fa664deaaaa13e85b2f7493f37f6144d86991ec012937307647bd3b9a82abe2974e1407241d54947bbb39763a4cac9f77166ad92a0@10.3.58.6:30303?discport=30301";
        let node: NodeRecord = url.parse().unwrap();
        let pk = node.pub_key;

        assert_eq!(node, NodeRecord {
            address: IpAddr::V4([10,3,58,6].into()),
            tcp_port: 30303,
            udp_port: 30301,
            id: "6f8a80d14311c39f35f516fa664deaaaa13e85b2f7493f37f6144d86991ec012937307647bd3b9a82abe2974e1407241d54947bbb39763a4cac9f77166ad92a0".parse().unwrap(),
            pub_key: pk,
            str: url.to_string(),
        
        })
    }
}
