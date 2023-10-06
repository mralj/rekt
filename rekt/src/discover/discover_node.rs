use std::net::Ipv4Addr;
use std::time::Instant;

use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::messages::discover_message::DEFAULT_MESSAGE_EXPIRATION;

pub(super) struct DiscoverNode {
    pub(super) node_record: NodeRecord,
    pub(super) ip_v4_addr: Ipv4Addr,

    pinged_on: Option<Instant>,
    ping_count: u8,
    pong_received_on: Option<Instant>,
}

impl DiscoverNode {
    #[inline(always)]
    pub(super) fn mark_ping_attempt(&mut self) {
        self.pinged_on = Some(Instant::now());
        self.ping_count += 1;
    }

    #[inline(always)]
    pub(super) fn pong_received(&mut self) {
        self.pong_received_on = Some(Instant::now());
        // we reset this so than future pings can be retired if need be
        self.ping_count = 0;
    }

    #[inline(always)]
    pub(super) fn udp_port(&self) -> u16 {
        self.node_record.udp_port
    }

    #[inline(always)]
    pub(super) fn id(&self) -> H512 {
        self.node_record.id
    }

    pub(super) fn re_ping_is_not_needed(&self) -> bool {
        if let Some(pinged_on) = self.pinged_on {
            if pinged_on.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION {
                return false;
            }
        }

        if let Some(pong_received_on) = self.pong_received_on {
            const HOURS_12: u64 = 60 * 60 * 12;
            if pong_received_on.elapsed().as_secs() < HOURS_12 {
                return false;
            }
        }

        self.ping_count > 3
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
        })
    }
}
