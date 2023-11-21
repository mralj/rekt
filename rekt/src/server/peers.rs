use std::net::IpAddr;

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

pub static PEERS_BY_IP: Lazy<DashMap<IpAddr, u8>> =
    Lazy::new(|| DashMap::with_capacity(2 * MAX_PEERS_UPPER_BOUND));

pub static BLACKLIST_PEERS_BY_ID: Lazy<DashSet<H512>> =
    Lazy::new(|| DashSet::with_capacity(MAX_PEERS_UPPER_BOUND));

pub static BLACKLIST_PEERS_BY_IP: Lazy<DashSet<IpAddr>> =
    Lazy::new(|| DashSet::with_capacity(MAX_PEERS_UPPER_BOUND));

pub fn check_if_already_connected_to_peer(node_record: &NodeRecord) -> Result<(), P2PError> {
    if PEERS.contains_key(&node_record.id) {
        return Err(P2PError::AlreadyConnected);
    }

    // if let Some(entry) = PEERS_BY_IP.get(&node_record.address) {
    //     if entry.value() >= &2 {
    //         return Err(P2PError::AlreadyConnectedToSameIp);
    //     }
    // }

    Ok(())
}

pub fn remove_peer_ip(ip: IpAddr) {
    match PEERS_BY_IP.entry(ip) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            if *entry.get() == 1 {
                entry.remove();
                return;
            }
            *entry.get_mut() -= 1;
        }
        _ => {}
    }
}

pub fn add_peer_ip(ip: IpAddr) {
    match PEERS_BY_IP.entry(ip) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            *entry.get_mut() += 1;
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

pub fn blacklist_peer(node_record: &NodeRecord) {
    BLACKLIST_PEERS_BY_ID.insert(node_record.id);
    //BLACKLIST_PEERS_BY_IP.insert(node_record.address);
}

pub fn peer_is_blacklisted(node_record: &NodeRecord) -> bool {
    BLACKLIST_PEERS_BY_ID.contains(&node_record.id)
    //|| BLACKLIST_PEERS_BY_IP.contains(&node_record.address)
}
