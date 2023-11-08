use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use futures::{SinkExt, TryStreamExt};
use kanal::AsyncSender;
use secp256k1::{PublicKey, SecretKey};
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Framed};
use tracing::error;

use crate::constants::DEFAULT_PORT;
use crate::p2p::errors::P2PError;
use crate::p2p::p2p_wire_message::P2pWireMessage;
use crate::p2p::peer::is_buy_or_sell_in_progress;
use crate::p2p::tx_sender::PEERS_SELL;
use crate::p2p::{self, HelloMessage, Peer, Protocol};
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::{RLPXMsg, RLPXMsgOut};
use crate::rlpx::errors::{RLPXError, RLPXSessionError};
use crate::rlpx::TcpWire;
use crate::rlpx::{utils::pk2id, Connection};
use crate::server::connection_task::ConnectionTask;
use crate::server::errors::ConnectionTaskError;
use crate::server::peers::{check_if_already_connected_to_peer, PEERS, PEERS_BY_IP};

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
        map_err!(check_if_already_connected_to_peer(&conn_task.node));

        let node = conn_task.node.clone();
        let rlpx_connection = Connection::new(secret_key, node.pub_key);

        // let socket = map_err!(TcpSocket::new_v4());
        // map_err!(socket.set_reuseport(true));
        // map_err!(socket.set_reuseaddr(true));
        //
        // map_err!(socket.bind(SocketAddr::V4(SocketAddrV4::new(
        //     Ipv4Addr::UNSPECIFIED,
        //     DEFAULT_PORT,
        // ))));

        let stream = map_err!(match timeout(
            Duration::from_secs(5),
            //socket.connect(node.get_socket_addr()),
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
        map_err!(transport.send(RLPXMsgOut::Auth).await);
        map_err!(handle_ack_msg(&mut transport).await);

        map_err!(
            transport
                .send(RLPXMsgOut::Message(
                    p2p::HelloMessage::get_our_hello_message(pk2id(&pub_key),)
                ))
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
            TcpWire::new(transport),
        );

        let task_result = p.run().await;
        if is_buy_or_sell_in_progress() {
            //NOTE: don't disconnect peers immediately to avoid UB (like nil ptr)

            while is_buy_or_sell_in_progress() {
                tokio::time::sleep(Duration::from_secs(120)).await;
            }
        }
        PEERS.remove(&node.id);
        PEERS_SELL.lock().await.remove(&node.id);

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
