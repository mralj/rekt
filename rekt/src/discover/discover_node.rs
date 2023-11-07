use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;

use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::messages::find_node::NeighborNodeRecord;
use super::messages::ping_pong_messages::PingMessage;

#[derive(Debug, Clone, PartialEq)]
pub enum DiscoverNodeType {
    Unknown,
    Static,
    WeDiscoveredThem,
    TheyDiscoveredUs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    NotAuthed,
    WeAuthedThem,
    TheyAuthedUs,
    Authed,
}

#[derive(Debug, Clone)]
pub struct DiscoverNode {
    pub node_record: NodeRecord,
    pub ip_v4_addr: Ipv4Addr,
    pub node_type: DiscoverNodeType,
    pub is_bsc_node: Option<bool>,

    pinged_on: Option<Instant>,
    ping_count: u8,

    pong_received_on: Option<Instant>,
    ping_received_on: Option<Instant>,
}

impl DiscoverNode {
    pub(super) fn auth_status(&self) -> AuthStatus {
        if self.we_have_authed_this_node() && self.this_node_has_authed_us() {
            return AuthStatus::Authed;
        }

        if self.we_have_authed_this_node() {
            return AuthStatus::WeAuthedThem;
        };

        if self.this_node_has_authed_us() {
            return AuthStatus::TheyAuthedUs;
        };

        AuthStatus::NotAuthed
    }

    #[inline(always)]
    pub(super) fn mark_ping_attempt(&mut self) {
        self.pinged_on = Some(Instant::now());
        self.ping_count += 1;
    }

    #[inline(always)]
    pub(super) fn mark_pong_received(&mut self) {
        self.pong_received_on = Some(Instant::now());
        // we reset this so than future pings can be retired if need be
        self.ping_count = 0;
    }

    #[inline(always)]
    pub(super) fn mark_ping_received(&mut self) {
        self.ping_received_on = Some(Instant::now());
    }

    #[inline(always)]
    pub(super) fn udp_port(&self) -> u16 {
        self.node_record.udp_port
    }

    #[inline(always)]
    pub(super) fn id(&self) -> H512 {
        self.node_record.id
    }

    pub(super) fn should_ping(&self, time_elapsed_for_ping_in_sec: u64) -> bool {
        if self.ping_count > 3 {
            return false;
        }

        if self.we_have_authed_this_node() {
            return false;
        }

        if let Some(pinged_on) = self.pinged_on {
            if pinged_on.elapsed().as_secs() < time_elapsed_for_ping_in_sec {
                return false;
            }
        }

        true
    }

    pub fn is_bsc(&self) -> bool {
        if let Some(is_bsc) = self.is_bsc_node {
            return is_bsc;
        }

        return false;
    }

    pub fn set_is_bsc(&mut self, is_bsc: bool) {
        self.is_bsc_node = Some(is_bsc);
    }

    pub fn should_blacklist(&self) -> bool {
        if self.auth_status() == AuthStatus::Authed {
            return false;
        }

        if self.pinged_on.is_none() {
            return false;
        }

        if self.ping_count < 3 {
            return false;
        }

        if let Some(pinged_on) = self.pinged_on {
            if pinged_on.elapsed().as_secs() < 60 {
                return false;
            }
        }

        self.ping_received_on.is_none() && self.pong_received_on.is_none()
    }

    pub(super) fn from_ping_msg(ping_msg: &PingMessage, id: H512) -> Result<Self, ()> {
        let node_record =
            NodeRecord::new_with_id(ping_msg.from.ip, ping_msg.from.tcp, ping_msg.from.udp, id)
                .map_err(|_| ())?;

        if let IpAddr::V4(ip) = ping_msg.from.ip {
            return Ok(Self {
                node_record,
                node_type: DiscoverNodeType::TheyDiscoveredUs,
                ip_v4_addr: ip,
                ping_received_on: Some(std::time::Instant::now()),
                ping_count: 0,
                pinged_on: None,
                pong_received_on: None,
                is_bsc_node: None,
            });
        }

        Err(())
    }

    fn we_have_authed_this_node(&self) -> bool {
        if let Some(pong_received_on) = self.pong_received_on {
            const HOURS_12: u64 = 60 * 60 * 12;
            if pong_received_on.elapsed().as_secs() < HOURS_12 {
                return true;
            }
        }

        false
    }

    fn this_node_has_authed_us(&self) -> bool {
        if let Some(ping_received_on) = self.ping_received_on {
            const HOURS_12: u64 = 60 * 60 * 12;
            if ping_received_on.elapsed().as_secs() < HOURS_12 {
                return true;
            }
        }

        false
    }
}

impl TryFrom<NodeRecord> for DiscoverNode {
    type Error = ();

    fn try_from(node_record: NodeRecord) -> Result<Self, Self::Error> {
        let ip_v4_addr = node_record.ip_v4_address().ok_or(())?;
        Ok(Self {
            node_record,
            ip_v4_addr,
            node_type: DiscoverNodeType::Static,
            pinged_on: None,
            ping_count: 0,
            pong_received_on: None,
            ping_received_on: None,
            is_bsc_node: Some(true),
        })
    }
}

impl TryFrom<NeighborNodeRecord> for DiscoverNode {
    type Error = ();

    fn try_from(value: NeighborNodeRecord) -> Result<Self, Self::Error> {
        let node_record =
            NodeRecord::new_with_id(value.address, value.tcp_port, value.udp_port, value.id)
                .map_err(|_| ())?;
        if let Some(ip) = node_record.ip_v4_address() {
            return Ok(Self {
                node_record,
                node_type: DiscoverNodeType::WeDiscoveredThem,
                ip_v4_addr: ip,
                pinged_on: None,
                ping_count: 0,
                pong_received_on: None,
                ping_received_on: None,
                is_bsc_node: None,
            });
        }

        Err(())
    }
}
