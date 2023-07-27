use std::io;
use std::time::Duration;

use futures::{SinkExt, TryStreamExt};
use kanal::AsyncSender;
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Framed};
use tracing::error;

use crate::p2p::errors::P2PError;
use crate::p2p::types::p2p_wire_message::P2pWireMessage;
use crate::p2p::types::{Peer, Protocol};
use crate::p2p::{self, HelloMessage};
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::{RLPXError, RLPXSessionError};
use crate::rlpx::TcpTransport;
use crate::rlpx::{utils::pk2id, Connection};
use crate::server::connection_task::ConnectionTask;
use crate::server::errors::ConnectionTaskError;
use crate::server::peers::{PEERS, PEERS_BY_IP};

pub fn connect_to_node(
    conn_task: ConnectionTask,
    secret_key: SecretKey,
    pub_key: PublicKey,
    tx: AsyncSender<ConnectionTaskError>,
) {
    tokio::spawn(async move {
        macro_rules! map_err {
            ($e: expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx
                            .send(ConnectionTaskError::new(
                                conn_task.next_attempt(),
                                RLPXSessionError::from(e),
                            ))
                            .await;
                        return;
                    }
                }
            };
        }
        if PEERS.contains_key(&conn_task.node.id) {
            map_err!(Err(P2PError::AlreadyConnected))
        }

        if PEERS_BY_IP.contains(&conn_task.node.ip) {
            map_err!(Err(P2PError::AlreadyConnectedToSameIp))
        }

        let node = conn_task.node.clone();
        let rlpx_connection = Connection::new(secret_key, node.pub_key);
        let stream = map_err!(match timeout(
            Duration::from_secs(5),
            TcpStream::connect(node.get_socket_addr()),
        )
        .await
        {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Connection timed out",
            )),
        });

        let mut transport = rlpx_connection.framed(stream);
        map_err!(transport.send(RLPXMsg::Auth).await);
        map_err!(handle_ack_msg(&mut transport).await);

        map_err!(
            transport
                .send(RLPXMsg::Message(p2p::HelloMessage::get_our_hello_message(
                    pk2id(&pub_key),
                )))
                .await
        );

        let (hello_msg, protocol_v) = map_err!(match handle_hello_msg(&mut transport).await {
            Ok(mut hello_msg) => {
                let matched_protocol =
                    map_err!(Protocol::match_protocols(&mut hello_msg.protocols)
                        .ok_or(RLPXSessionError::NoMatchingProtocols));

                Ok((hello_msg, matched_protocol.version))
            }
            Err(e) => Err(e),
        });

        let mut p = Peer::new(
            node.clone(),
            hello_msg.id,
            protocol_v,
            hello_msg.client_version,
            TcpTransport::new(transport),
        );

        let task_result = p.run().await;
        PEERS.remove(&node.id);

        // In case we got already connected to same ip error we do not remove the IP from the set
        // of already connected ips
        // But in all other cases we must remove the IP from the set
        if !matches!(task_result, Err(P2PError::AlreadyConnectedToSameIp)) {
            PEERS_BY_IP.remove(&node.ip);
        }

        map_err!(task_result);
    });
}

async fn handle_ack_msg(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<(), RLPXSessionError> {
    let msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidAckData)?;

    if !matches!(msg, RLPXMsg::Ack) {
        error!("Got unexpected message: {:?}", msg);
        return Err(RLPXSessionError::UnexpectedMessage {
            received: msg,
            expected: RLPXMsg::Ack,
        });
    }
    Ok(())
}

async fn handle_hello_msg(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<HelloMessage, RLPXSessionError> {
    let rlpx_msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidMsgData)?;

    if let RLPXMsg::Message(rlpx_msg) = rlpx_msg {
        let msg = P2pWireMessage::new(rlpx_msg)?;
        let p2p_msg = P2PMessage::decode(msg.id, &mut &msg.data[..])?;

        match p2p_msg {
            P2PMessage::Hello(node_info) => {
                return Ok(node_info);
            }
            P2PMessage::Disconnect(reason) => {
                return Err(RLPXSessionError::DisconnectRequested(reason));
            }
            _ => {
                return Err(RLPXSessionError::UnexpectedP2PMessage {
                    received: msg.id,
                    expected: P2PMessageID::Hello as u8,
                });
            }
        };
    }

    error!("Not RLPX: Got unexpected message: {:?}", rlpx_msg);
    Err(RLPXSessionError::ExpectedRLPXMessage)
}
