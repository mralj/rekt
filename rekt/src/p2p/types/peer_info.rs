use std::fmt::{Display, Formatter};

use crate::types::hash::H512;

use super::P2PPeer;

#[derive(Debug)]
pub struct PeerInfo {
    pub id: H512,
    pub info: String,
    pub enode: String,
    pub ip: String,
}

impl From<&P2PPeer> for PeerInfo {
    fn from(p: &P2PPeer) -> Self {
        Self {
            id: p.id,
            info: p.info.clone(),
            enode: p.node_record.str.clone(),
            ip: p.node_record.ip.clone(),
        }
    }
}

impl Display for PeerInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IP: {}, INFO: {}", self.ip, self.info)
    }
}
