use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use tokio::time::interval;
use tokio_stream::StreamExt;

use crate::{
    discover::{
        discover_node::{AuthStatus, DiscoverNode},
        server::Server,
    },
    types::hash::H512,
};

const ALPHA: usize = 100;

impl Server {
    pub fn get_next_lookup_id(&self) -> H512 {
        H512::random()
    }

    pub fn get_closest_nodes(&self, lookup_id: H512) -> Vec<DiscoverNode> {
        let mut nodes = self
            .nodes
            .iter()
            //NOTE: maybe just return BSC nodes?
            // think about this when we implement requesting ENR
            .filter(|n| n.auth_status() == AuthStatus::Authed)
            .map(|n| n.value().clone())
            .collect::<Vec<DiscoverNode>>();

        nodes.sort_by(|a, b| {
            let a_distance = a.id().distance(&lookup_id);
            let b_distance = b.id().distance(&lookup_id);

            a_distance.cmp(&b_distance)
        });

        nodes.into_iter().take(ALPHA).collect()
    }

    pub async fn run_lookup(&self) {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(7),
        ));

        while let Some(_) = stream.next().await {
            if self.is_paused() {
                continue;
            }
            let pending_lookups_to_retain = self
                .pending_neighbours_req
                .iter()
                .map(|v| v.lookup_id)
                .collect::<Vec<_>>();

            self.pending_lookups
                .retain(|k, _| pending_lookups_to_retain.contains(k));

            let next_lookup_id = self.get_next_lookup_id();
            let closest_nodes = self.get_closest_nodes(next_lookup_id);
            self.pending_lookups.insert(
                next_lookup_id,
                Lookup::new(next_lookup_id, closest_nodes.clone()),
            );

            for n in closest_nodes.iter() {
                self.pending_neighbours_req
                    .insert(n.id(), PendingNeighboursReq::new(next_lookup_id, n));
            }
            let tasks = FuturesUnordered::from_iter(closest_nodes.iter().map(|n| {
                self.send_neighbours_packet(next_lookup_id, (n.ip_v4_addr, n.udp_port()))
            }));

            let _result = tasks.collect::<Vec<_>>().await;
        }
    }
}

pub struct Lookup {
    pub lookup_id: H512,
    pub closest_nodes: BTreeMap<H512, LookupNode>,
    pub queried_count: usize,
    pub responded_count: usize,
}

impl Lookup {
    pub fn new(lookup_id: H512, closest_nodes: Vec<DiscoverNode>) -> Self {
        let closest_nodes = closest_nodes
            .into_iter()
            .map(|n| {
                let mut node = LookupNode::from(n);
                node.request_sent = true;
                (node.node.id().distance(&lookup_id), node)
            })
            .collect::<BTreeMap<H512, LookupNode>>();

        Self {
            lookup_id,
            closest_nodes,
            queried_count: 0,
            responded_count: 0,
        }
    }

    pub fn mark_node_responded(&mut self, node_id: H512) {
        if let Some((_k, node)) = self
            .closest_nodes
            .iter_mut()
            .find(|(_, n)| n.node.id() == node_id)
        {
            node.responded = true;
        }

        self.responded_count += 1;
    }

    pub fn add_new_nodes(&mut self, nodes: Vec<DiscoverNode>) {
        nodes.into_iter().for_each(|node| {
            if let Entry::Vacant(entry) = self
                .closest_nodes
                .entry(node.id().distance(&self.lookup_id))
            {
                entry.insert(node.into());
            }
        });
    }

    pub fn get_next_nodes_to_query(
        &mut self,
        nodes: &DashMap<H512, DiscoverNode>,
    ) -> Vec<DiscoverNode> {
        self.closest_nodes
            .iter_mut()
            .filter(|(_, n)| !n.request_sent && !n.responded)
            .take(ALPHA)
            .map(|(_, n)| {
                n.request_sent = true;
                //so that we get latest node status
                if let Some(node) = nodes.get(&n.node.id()) {
                    node.value().clone()
                } else {
                    n.node.clone()
                }
            })
            .collect::<Vec<DiscoverNode>>()
    }
}

pub struct LookupNode {
    pub node: DiscoverNode,
    pub request_sent: bool,
    pub responded: bool,
}

impl From<DiscoverNode> for LookupNode {
    fn from(node: DiscoverNode) -> Self {
        Self {
            node,
            request_sent: false,
            responded: false,
        }
    }
}

pub struct PendingNeighboursReq {
    pub lookup_id: H512,
    pub node_id: H512,
    pub created_on: std::time::Instant,
    pub was_authed: bool,
    pub ip: std::net::Ipv4Addr,
    pub udp: u16,
}

impl PendingNeighboursReq {
    pub fn new(lookup_id: H512, node: &DiscoverNode) -> Self {
        Self {
            lookup_id,
            created_on: std::time::Instant::now(),
            node_id: node.id(),
            ip: node.ip_v4_addr,
            udp: node.udp_port(),
            was_authed: node.auth_status() == AuthStatus::Authed
                || node.auth_status() == AuthStatus::TheyAuthedUs,
        }
    }
}
