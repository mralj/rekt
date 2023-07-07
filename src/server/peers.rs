use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;

use crate::types::hash::H512;

// we've never connected above 2.5k peers, especially now that we blacklist IPs
const MAX_PEERS_UPPER_BOUND: usize = 2_500;

pub static PEERS: Lazy<DashMap<H512, String>> =
    Lazy::new(|| DashMap::with_capacity(2 * MAX_PEERS_UPPER_BOUND));

pub static PEERS_BY_IP: Lazy<DashSet<String>> =
    Lazy::new(|| DashSet::with_capacity(2 * MAX_PEERS_UPPER_BOUND));

pub static BLACKLIST_PEERS_BY_ID: Lazy<DashSet<H512>> =
    Lazy::new(|| DashSet::with_capacity(MAX_PEERS_UPPER_BOUND));
