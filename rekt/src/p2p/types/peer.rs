use std::fmt::{Display, Formatter};

use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::error;

use super::errors::P2PError;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use crate::eth;
use crate::eth::protocol::EthMessages;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::p2p::p2p_wire::P2PWire;
use crate::rlpx::TcpTransport;
use crate::server::peers::{check_if_already_connected_to_peer, PEERS, PEERS_BY_IP};
use crate::types::hash::H512;

use crate::types::node_record::NodeRecord;

#[derive(Debug)]
pub struct P2PPeer {
    pub id: H512,
    pub(crate) node_record: NodeRecord,
    pub(crate) info: String,
    protocol_version: ProtocolVersion,
    connection: P2PWire,
}

impl P2PPeer {
    pub fn new(
        enode: NodeRecord,
        id: H512,
        protocol: usize,
        info: String,
        connection: TcpTransport,
    ) -> Self {
        Self {
            id,
            connection: P2PWire::new(connection),
            info,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
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
        check_if_already_connected_to_peer(&self.node_record)?;
        self.handshake().await?;
        check_if_already_connected_to_peer(&self.node_record)?;

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
            let _ = eth::msg_handler::handle_eth_message(msg);
        }
    }

    pub async fn send_our_status_msg(&mut self) -> Result<(), P2PError> {
        self.connection
            .send(Status::get(&self.protocol_version))
            .await?;

        self.connection.send(UpgradeStatus::get()).await
    }

    async fn handshake(&mut self) -> Result<(), P2PError> {
        let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;

        if EthMessages::from(msg.id.unwrap()) != EthMessages::StatusMsg {
            error!("Expected status message, got {:?}", msg.id);
            return Err(P2PError::ExpectedStatusMessage);
        }

        let status_msg = Status::decode(&mut &msg.data[..])?;

        if Status::validate(&status_msg, &self.protocol_version).is_err() {
            return Err(P2PError::CouldNotValidateStatusMessage);
        }

        self.connection
            .send(Status::get(&self.protocol_version))
            .await?;

        self.handle_upgrade_status_messages().await
    }

    async fn handle_upgrade_status_messages(&mut self) -> Result<(), P2PError> {
        if self.protocol_version == ProtocolVersion::Eth66 {
            return Ok(());
        }

        self.connection.send(UpgradeStatus::get()).await?;
        let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;
        if EthMessages::from(msg.id.unwrap()) != EthMessages::UpgradeStatusMsg {
            return Err(P2PError::ExpectedUpgradeStatusMessage);
        }

        Ok(())
    }
}
