use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;

use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::messages::ping_pong_messages::PingMessage;

pub(super) struct DiscoverNode {
    pub(super) node_record: NodeRecord,
    pub(super) ip_v4_addr: Ipv4Addr,

    pinged_on: Option<Instant>,
    ping_count: u8,

    pong_received_on: Option<Instant>,
    ping_received_on: Option<Instant>,
}

impl DiscoverNode {
    pub(super) fn we_have_authed_this_node(&self) -> bool {
        if let Some(pong_received_on) = self.pong_received_on {
            const HOURS_12: u64 = 60 * 60 * 12;
            if pong_received_on.elapsed().as_secs() < HOURS_12 {
                return true;
            }
        }

        false
    }

    pub(super) fn this_node_has_authed_us(&self) -> bool {
        if let Some(ping_received_on) = self.ping_received_on {
            const HOURS_12: u64 = 60 * 60 * 12;
            if ping_received_on.elapsed().as_secs() < HOURS_12 {
                return true;
            }
        }

        false
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

    pub(super) fn from_ping_msg(ping_msg: &PingMessage, id: H512) -> Result<Self, ()> {
        let node_record =
            NodeRecord::new_with_id(ping_msg.from.ip, ping_msg.from.tcp, ping_msg.from.udp, id)
                .map_err(|_| ())?;

        if let IpAddr::V4(ip) = ping_msg.from.ip {
            return Ok(Self {
                node_record,
                ip_v4_addr: ip,
                ping_received_on: Some(std::time::Instant::now()),
                ping_count: 0,
                pinged_on: None,
                pong_received_on: None,
            });
        }

        Err(())
    }
}

impl TryFrom<NodeRecord> for DiscoverNode {
    type Error = ();

    fn try_from(node_record: NodeRecord) -> Result<Self, Self::Error> {
        let ip_v4_addr = node_record.ip_v4_address().ok_or(())?;
        Ok(Self {
            node_record,
            ip_v4_addr,
            pinged_on: None,
            ping_count: 0,
            pong_received_on: None,
            ping_received_on: None,
        })
    }
}
