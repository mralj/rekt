use std::fmt::{Display, Formatter};

use crate::types::hash::H512;

use super::capability::{CapVersion, CapVersionError};

#[derive(Debug)]
pub struct P2PPeer {
    enode: String,
    id: H512,
    capability: CapVersion,
}

impl P2PPeer {
    pub fn new(enode: String, id: H512, capability: usize) -> Result<Self, CapVersionError> {
        let capability = CapVersion::try_from(capability)?;
        Ok(Self {
            enode,
            id,
            capability,
        })
    }
}

impl Display for P2PPeer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, capability: {}",
            self.enode, self.id, self.capability
        )
    }
}
