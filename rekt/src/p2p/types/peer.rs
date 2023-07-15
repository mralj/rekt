use core::time;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use futures::stream::{SplitSink, SplitStream};
use futures::{pin_mut, SinkExt, StreamExt};

use kanal::{AsyncReceiver, AsyncSender};
use open_fastrlp::{Decodable, Encodable};
use tokio::select;
use tracing::{error, info};

use super::errors::P2PError;
use super::p2p_wire::P2PWire;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use crate::eth;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::p2p::P2PMessage;
use crate::server::peers::{PEERS, PEERS_BY_IP};
use crate::types::hash::H512;

use crate::types::message::{Message, MessageKind};
use crate::types::node_record::NodeRecord;

#[derive(Debug)]
pub struct P2PPeer {
    pub id: H512,
    pub(crate) node_record: NodeRecord,
    pub(crate) info: String,
    protocol_version: ProtocolVersion,
    connected_on: Instant,
    msg_rx: AsyncReceiver<Message>,
    msg_tx: AsyncSender<Message>,
}

impl P2PPeer {
    pub fn new(enode: NodeRecord, id: H512, protocol: usize, info: String) -> Self {
        let (msg_tx, msg_rx) = kanal::unbounded_async();

        Self {
            id,
            info,
            msg_rx,
            msg_tx,
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
    pub async fn run(peer: Arc<P2PPeer>, wire: P2PWire) -> Result<(), P2PError> {
        let (w, r) = wire.split();
        let r_task = P2PPeer::run_reader(peer.clone(), r);
        let w_task = P2PPeer::run_writer(peer.clone(), w);

        pin_mut!(w_task, r_task); // needed for `select!`

        select! {
            r = r_task => {
                peer.msg_tx.close();
                peer.msg_rx.close();
                r
            },
            r = w_task => {
                peer.msg_tx.close();
                peer.msg_rx.close();
                r
            },
        }
    }

    async fn run_writer(
        peer: Arc<P2PPeer>,
        mut writer: SplitSink<P2PWire, Message>,
    ) -> Result<(), P2PError> {
        match tokio::spawn(async move {
            while let Some(m) = peer.msg_rx.stream().next().await {
                let msg_to_write = match m.kind {
                    Some(MessageKind::ETH) => {
                        eth::msg_handler::handle_eth_message(m).map_err(P2PError::EthError)
                    }
                    Some(MessageKind::P2P(p2p_m)) => P2PPeer::handle_p2p_msg(&p2p_m),
                    _ => Ok(None),
                };
                if let Ok(Some(m)) = msg_to_write {
                    writer.send(m).await?
                }
            }

            Err(P2PError::NoMessage)
        })
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(P2PError::Unknown),
        }
    }

    async fn run_reader(
        peer: Arc<P2PPeer>,
        mut reader: SplitStream<P2PWire>,
    ) -> Result<(), P2PError> {
        match tokio::spawn(async move {
            loop {
                let msg = reader
                    .next()
                    .await
                    // by stream definition when Poll::Ready(None) is returned this means that
                    // stream is done and should not be polled again, or bad things will happen
                    .ok_or(P2PError::NoMessage)??; //

                //handle messages only after 5m to reduce old TXs
                if Instant::now().duration_since(peer.connected_on)
                    <= time::Duration::from_secs(5 * 60)
                {
                    continue;
                }

                peer.msg_tx.send(msg).await.map_err(|_| P2PError::Unknown)?;
            }
        })
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(P2PError::Unknown),
        }
    }
    pub(crate) async fn handshake(peer: Arc<P2PPeer>, wire: &mut P2PWire) -> Result<(), P2PError> {
        if PEERS_BY_IP.contains(&peer.node_record.ip) {
            return Err(P2PError::AlreadyConnectedToSameIp);
        }

        if PEERS.contains_key(&peer.node_record.id) {
            return Err(P2PError::AlreadyConnected);
        }

        let msg = wire.next().await.ok_or(P2PError::NoMessage)??;

        if msg.id != Some(16) {
            error!("Expected status message, got {:?}", msg.id);
            return Err(P2PError::ExpectedStatusMessage);
        }

        let status_msg = Status::decode(&mut &msg.data[..])?;

        if Status::validate(&status_msg, &peer.protocol_version).is_err() {
            return Err(P2PError::CouldNotValidateStatusMessage);
        }

        P2PPeer::handle_status_messages(peer.clone(), wire).await?;

        if PEERS_BY_IP.contains(&peer.node_record.ip) {
            return Err(P2PError::AlreadyConnectedToSameIp);
        }

        if PEERS.contains_key(&peer.node_record.id) {
            return Err(P2PError::AlreadyConnected);
        }

        PEERS.insert(peer.node_record.id, PeerInfo::from(&peer as &P2PPeer));
        PEERS_BY_IP.insert(peer.node_record.ip.clone());

        Ok(())
    }

    pub(crate) async fn handle_status_messages(
        peer: Arc<P2PPeer>,
        wire: &mut P2PWire,
    ) -> Result<(), P2PError> {
        wire.send(Status::get(&peer.protocol_version)).await?;

        if peer.protocol_version == ProtocolVersion::Eth67 {
            wire.send(UpgradeStatus::get()).await?;
            let msg = wire.next().await.ok_or(P2PError::NoMessage)??;
            if msg.id != Some(27) {
                return Err(P2PError::ExpectedUpgradeStatusMessage);
            }
        }

        Ok(())
    }

    fn handle_p2p_msg(msg: &P2PMessage) -> Result<Option<Message>, P2PError> {
        match msg {
            P2PMessage::Ping => {
                let mut buf = BytesMut::new();
                P2PMessage::Pong.encode(&mut buf);

                Ok(Some(Message {
                    id: Some(2),
                    kind: Some(MessageKind::P2P(P2PMessage::Pong)),
                    data: buf,
                }))
            }
            P2PMessage::Disconnect(r) => Err(P2PError::DisconnectRequested(*r)),
            P2PMessage::Hello(_) => Err(P2PError::UnexpectedHelloMessageReceived), // TODO: proper err here
            // NOTE: this is no-op for us, technically we should never
            // receive pong message as we don't send Pings, but we'll just ignore it
            P2PMessage::Pong => Ok(None),
        }
    }
}
