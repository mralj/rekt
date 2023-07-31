use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;

use crate::p2p::errors::P2PError;
use crate::p2p::peer_info::PeerInfo;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

// we've never connected above 2.5k peers, especially now that we blacklist IPs
const MAX_PEERS_UPPER_BOUND: usize = 2_500;

pub static PEERS: Lazy<DashMap<H512, PeerInfo>> =
    Lazy::new(|| DashMap::with_capacity(2 * MAX_PEERS_UPPER_BOUND));

pub static PEERS_BY_IP: Lazy<DashSet<String>> =
    Lazy::new(|| DashSet::with_capacity(2 * MAX_PEERS_UPPER_BOUND));

pub static BLACKLIST_PEERS_BY_ID: Lazy<DashSet<H512>> =
    Lazy::new(|| DashSet::with_capacity(MAX_PEERS_UPPER_BOUND));

pub fn check_if_already_connected_to_peer(node_record: &NodeRecord) -> Result<(), P2PError> {
    if PEERS_BY_IP.contains(&node_record.ip) {
        return Err(P2PError::AlreadyConnectedToSameIp);
    }

    if PEERS.contains_key(&node_record.id) {
        return Err(P2PError::AlreadyConnected);
    }

    Ok(())
}
