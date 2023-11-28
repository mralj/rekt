use std::net::{IpAddr, Ipv4Addr};

use bytes::BytesMut;
use enr::{Enr, EnrBuilder};
use open_fastrlp::Encodable;
use secp256k1::{PublicKey, SecretKey};

use crate::blockchain::fork::ForkId;
use crate::constants::DEFAULT_PORT;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

#[derive(Debug, Clone)]
pub struct LocalNode {
    pub node_record: NodeRecord,
    pub private_key: secp256k1::SecretKey,
    pub public_key: PublicKey,
    pub public_ip_retrieved: bool,
    pub enr: Enr<SecretKey>,
    pub public_ip: Option<IpAddr>,
}

impl LocalNode {
    pub fn new(ip: Option<IpAddr>) -> Self {
        let private_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &private_key);
        let (ip, public_ip_retrieved) = match ip {
            Some(ip) => (ip, true),
            None => (IpAddr::V4(Ipv4Addr::UNSPECIFIED), false),
        };

        let fork_id_rlp_encoded = {
            #[derive(open_fastrlp::RlpEncodable)]
            struct ForkIdRlpHelper {
                fork_id: ForkId,
            }

            let mut buf = BytesMut::new();
            let fork_id = *crate::blockchain::bsc_chain_spec::BSC_MAINNET_FORK_ID;

            let fork_id_rlp_helper = ForkIdRlpHelper { fork_id };
            fork_id_rlp_helper.encode(&mut buf);

            buf.freeze()
        };

        let local_enr = EnrBuilder::new("v4")
            .ip(ip)
            .udp4(DEFAULT_PORT)
            .tcp4(DEFAULT_PORT)
            .add_value_rlp("eth", fork_id_rlp_encoded)
            .build(&private_key)
            .unwrap();

        Self {
            private_key,
            public_key,
            public_ip_retrieved,
            enr: local_enr,
            node_record: NodeRecord::new(ip, DEFAULT_PORT, DEFAULT_PORT, public_key),
            public_ip: Some(ip),
        }
    }
}
