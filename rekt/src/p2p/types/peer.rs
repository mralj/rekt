use core::time;
use std::fmt::{Display, Formatter};
use std::task::Poll;
use std::time::Instant;

use futures::{Future, SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::{error, info};

use super::errors::P2PError;
use super::p2p_wire::P2PWire;
use super::protocol::ProtocolVersion;
use crate::eth;
use crate::eth::msg_handler::handle_eth_message;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::server::peers::{PEERS, PEERS_BY_IP};
use crate::types::hash::H512;

use crate::types::message::Message;
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
    pub async fn handshake(&mut self) -> Result<(), P2PError> {
        if PEERS_BY_IP.contains(&self.node_record.ip) {
            return Err(P2PError::AlreadyConnectedToSameIp);
        }

        if PEERS.contains_key(&self.node_record.id) {
            return Err(P2PError::AlreadyConnected);
        }

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

        if PEERS_BY_IP.contains(&self.node_record.ip) {
            return Err(P2PError::AlreadyConnectedToSameIp);
        }

        if PEERS.contains_key(&self.node_record.id) {
            return Err(P2PError::AlreadyConnected);
        }
        Ok(())
    }
}

impl Future for P2PPeer {
    type Output = Result<(), P2PError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        loop {
            let msg: Result<Option<Message>, P2PError> = match self.connection.poll_next_unpin(cx) {
                Poll::Pending => Ok(None),
                Poll::Ready(None) => {
                    return Poll::Ready(Err(P2PError::NoMessage));
                }
                Poll::Ready(Some(Ok(msg))) => {
                    handle_eth_message(msg, self.connected_on).map_err(P2PError::EthError)
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(e));
                }
            };

            if let Err(e) = msg {
                return Poll::Ready(Err(e));
            }

            match self.connection.poll_ready_unpin(cx) {
                Poll::Pending => match self.connection.poll_flush_unpin(cx) {
                    Poll::Pending => continue,
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Err(e));
                    }
                    Poll::Ready(Ok(_)) => {
                        if let Some(m) = msg.unwrap() {
                            self.connection.start_send_unpin(m);
                        }
                    }
                },
                Poll::Ready(Ok(_)) => {
                    if let Some(m) = msg.unwrap() {
                        self.connection.start_send_unpin(m);
                    }
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e));
                }
            };

            if self.connection.has_messages_queued() {
                match self.connection.poll_flush_unpin(cx) {
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Err(e));
                    }
                    _ => continue,
                }
            } else {
                return Poll::Pending;
            }
        }
    }
}
