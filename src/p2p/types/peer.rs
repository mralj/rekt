use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};

use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::{error, info};

use super::p2p_wire::P2PWire;
use super::protocol::ProtocolVersion;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::rlpx::RLPXSessionError;
use crate::types::hash::H512;
use crate::types::message::Message;
use crate::types::node_record::NodeRecord;

#[derive(Debug)]
pub struct P2PPeer {
    node_record: NodeRecord,
    pub id: H512,
    protocol_version: ProtocolVersion,
    connection: P2PWire,
}

impl P2PPeer {
    pub fn new(enode: NodeRecord, id: H512, protocol: usize, connection: P2PWire) -> Self {
        Self {
            id,
            connection,
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
    pub async fn run(&mut self, peers: Arc<Mutex<HashSet<H512>>>) -> Result<(), RLPXSessionError> {
        self.handshake().await?;
        {
            peers.lock().unwrap().insert(self.id);
        }

        loop {
            let msg = self
                .connection
                .next()
                .await
                // by stream definition when Poll::Ready(None) is returned this means that
                // stream is done and should not be polled again, or bad things will happen
                .ok_or(RLPXSessionError::NoMessage)??; //
            self.handle_eth_message(msg).await?;
        }
    }

    pub async fn send_our_status_msg(&mut self) -> Result<(), RLPXSessionError> {
        self.connection
            .send(Status::get(&self.protocol_version))
            .await?;

        self.connection.send(UpgradeStatus::get()).await
    }

    async fn handle_eth_message(&mut self, msg: Message) -> Result<(), RLPXSessionError> {
        let msg_id_is_bsc_upgrade_status_msg = msg.id == Some(27);
        if !msg_id_is_bsc_upgrade_status_msg {
            //  info!("Got ETH message with ID: {:?}", msg.id);
        } else {
            info!("Got upgrade status msg");
        }

        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), RLPXSessionError> {
        let msg = self
            .connection
            .next()
            .await
            .ok_or(RLPXSessionError::NoMessage)??;

        if msg.id != Some(16) {
            error!("Expected status message, got {:?}", msg.id);
            return Err(RLPXSessionError::UnknownError);
        }

        let status_msg = Status::decode(&mut &msg.data[..])?;

        if Status::validate(&status_msg, &self.protocol_version).is_err() {
            return Err(RLPXSessionError::UnknownError);
        } else {
            info!("Validated status MSG OK");
        }

        self.send_our_status_msg().await
    }
}
