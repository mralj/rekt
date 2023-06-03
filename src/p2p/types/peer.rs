use std::fmt::{Display, Formatter};

use crate::types::hash::H512;

use super::protocol::{ProtocolVersion, ProtocolVersionError};

#[derive(Debug)]
pub struct P2PPeer {
    enode: String,
    id: H512,
    protocol_version: ProtocolVersion,
}

impl P2PPeer {
    pub fn new(enode: String, id: H512, protocol: usize) -> Result<Self, ProtocolVersionError> {
        let protocol = ProtocolVersion::try_from(protocol)?;
        Ok(Self {
            enode,
            id,
            protocol_version: protocol,
        })
    }
}

impl Display for P2PPeer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.enode, self.id, self.protocol_version
        )
    }
}
