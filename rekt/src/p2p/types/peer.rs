use core::time;
use std::fmt::{Display, Formatter};
use std::time::Instant;

use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::{error, info};

use super::errors::P2PError;
use super::p2p_wire::P2PWire;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use crate::eth;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::server::peers::{PEERS, PEERS_BY_IP};
use crate::types::hash::H512;

use crate::types::node_record::NodeRecord;

#[derive(Debug)]
pub struct P2PPeer {
    pub id: H512,
    pub(crate) node_record: NodeRecord,
    pub(crate) info: String,
    protocol_version: ProtocolVersion,
    connection: P2PWire,
    connected_on: Instant,
}

impl P2PPeer {
    pub fn new(
        enode: NodeRecord,
        id: H512,
        protocol: usize,
        info: String,
        connection: P2PWire,
    ) -> Self {
        Self {
            id,
            connection,
            info,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
            connected_on: Instant::now(),
        }
    }
}

impl Display for P2PPeer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.node_record.str, self.id, self.protocol_version
        )
    }
}

impl P2PPeer {
    pub async fn run(&mut self) -> Result<(), P2PError> {
        self.handshake().await?;

        if PEERS_BY_IP.contains(&self.node_record.ip) {
            return Err(P2PError::AlreadyConnectedToSameIp);
        }

        if PEERS.contains_key(&self.node_record.id) {
            return Err(P2PError::AlreadyConnected);
        }

        PEERS.insert(self.node_record.id, PeerInfo::from(self as &P2PPeer));
        PEERS_BY_IP.insert(self.node_record.ip.clone());

        loop {
            let msg = self
                .connection
                .next()
                .await
                // by stream definition when Poll::Ready(None) is returned this means that
                // stream is done and should not be polled again, or bad things will happen
                .ok_or(P2PError::NoMessage)??; //

            //handle messages only after 10m to reduce old TXs
            if Instant::now().duration_since(self.connected_on)
                <= time::Duration::from_secs(10 * 60)
            {
                continue;
            }

            let r = eth::msg_handler::handle_eth_message(msg)?;
            if let Some(r) = r {
                self.connection.send(r).await?;
            }
        }
    }

    pub async fn handshake(&mut self) -> Result<(), P2PError> {
        let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;

        if msg.id != Some(16) {
            error!("Expected status message, got {:?}", msg.id);
            return Err(P2PError::ExpectedStatusMessage);
        }

        let status_msg = Status::decode(&mut &msg.data[..])?;

        if Status::validate(&status_msg, &self.protocol_version).is_err() {
            return Err(P2PError::CouldNotValidateStatusMessage);
        }

        self.handle_status_messages().await
    }

    pub async fn handle_status_messages(&mut self) -> Result<(), P2PError> {
        self.connection
            .send(Status::get(&self.protocol_version))
            .await?;

        if self.protocol_version == ProtocolVersion::Eth67 {
            self.connection.send(UpgradeStatus::get()).await?;
            let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;
            if msg.id != Some(27) {
                return Err(P2PError::ExpectedUpgradeStatusMessage);
            }
        }

        Ok(())
    }
}
