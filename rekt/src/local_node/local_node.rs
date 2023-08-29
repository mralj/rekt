use std::net::{IpAddr, Ipv4Addr};

use secp256k1::{PublicKey, SecretKey};

use crate::constants::DEFAULT_PORT;
use crate::types::node_record::NodeRecord;

pub struct LocalNode {
    pub node_record: NodeRecord,
    pub private_key: SecretKey,
    pub public_key: PublicKey,
    pub public_ip_retrieved: bool,
}

impl LocalNode {
    pub fn new(ip: Option<IpAddr>) -> Self {
        let private_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key =
            secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &private_key);
        let (ip, public_ip_retrieved) = match ip {
            Some(ip) => (ip, true),
            None => (IpAddr::V4(Ipv4Addr::UNSPECIFIED), false),
        };

        Self {
            private_key,
            public_key,
            public_ip_retrieved,
            node_record: NodeRecord::new(ip, DEFAULT_PORT, DEFAULT_PORT, public_key),
        }
    }
}
