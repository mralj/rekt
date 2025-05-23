use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, TryStreamExt};
use secp256k1::PublicKey;
use tokio::net::TcpStream;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Framed};
use tracing::error;

use crate::eth::eth_message::EthMessage;
use crate::p2p::errors::P2PError;
use crate::p2p::p2p_wire_message::P2pWireMessage;
use crate::p2p::peer::{is_buy_or_sell_in_progress, PeerType};
use crate::p2p::{self, HelloMessage, Peer, Protocol};
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::{RLPXMsg, RLPXMsgOut};
use crate::rlpx::errors::{RLPXError, RLPXSessionError};
use crate::rlpx::TcpWire;
use crate::rlpx::{utils::pk2id, Connection};
use crate::server::connection_task::ConnectionTask;
use crate::server::errors::ConnectionTaskError;
use crate::server::peers::{check_if_already_connected_to_peer, PEERS, PEERS_BY_IP};

pub async fn connect_to_node(
    conn_task: ConnectionTask,
    tx: UnboundedSender<ConnectionTaskError>,
    conn_permit: Arc<tokio::sync::Semaphore>,
    tx_sender: Sender<EthMessage>,
) {
    let permit = conn_permit.clone().acquire_owned().await.unwrap();
    tokio::spawn(async move {
        macro_rules! map_err {
            ($e: expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.send(ConnectionTaskError::new(
                            conn_task.next_attempt(),
                            RLPXSessionError::from(e),
                        ));
                        return;
                    }
                }
            };
        }
        map_err!(check_if_already_connected_to_peer(&conn_task.node));

        let node = conn_task.node.clone();
        let rlpx_connection = Connection::new_out(conn_task.our_sk, node.pub_key);

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

        if let Err(e) = stream.set_nodelay(true) {
            println!("Set no delay err: {}", e);
        }

        let mut transport = rlpx_connection.framed(stream);
        map_err!(handle_auth(&mut transport).await);

        let (hello_msg, protocol_v) =
            map_err!(
                match handle_hello_msg(&conn_task.our_pk, &mut transport).await {
                    Ok(mut hello_msg) => {
                        let matched_protocol =
                            map_err!(Protocol::match_protocols(&mut hello_msg.protocols)
                                .ok_or(RLPXSessionError::NoMatchingProtocols));

                        Ok((hello_msg, matched_protocol.version))
                    }
                    Err(e) => Err(e),
                }
            );

        //conn attempt succeeded, so we can release the permit
        drop(permit);

        let mut p = Peer::new(
            node.clone(),
            hello_msg.id,
            protocol_v,
            hello_msg.client_version,
            TcpWire::new(transport),
            PeerType::Outbound,
            conn_task.server_info.clone(),
            tx_sender,
        );

        let task_result = p.run().await;
        if is_buy_or_sell_in_progress() {
            //NOTE: don't disconnect peers immediately to avoid UB (like nil ptr)

            while is_buy_or_sell_in_progress() {
                tokio::time::sleep(Duration::from_secs(120)).await;
            }
        }
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

async fn handle_auth(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<(), RLPXSessionError> {
    transport.send(RLPXMsgOut::Auth).await?;
    let msg = match transport.try_next().await {
        Ok(Some(msg)) => msg,
        Ok(None) => return Err(RLPXSessionError::ConnectionClosed),
        Err(e) => return Err(RLPXSessionError::RlpxError(e)),
    };
    //.ok_or(RLPXError::InvalidAckData)?;

    if !matches!(msg, RLPXMsg::Ack) {
        error!("Got unexpected message: {:?}", msg);
        return Err(RLPXSessionError::UnexpectedMessage {
            received: msg,
            expected: RLPXMsg::Ack,
        });
    }
    Ok(())
}

pub(super) async fn handle_hello_msg(
    pub_key: &PublicKey,
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<HelloMessage, RLPXSessionError> {
    transport
        .send(RLPXMsgOut::Message(
            p2p::HelloMessage::get_our_hello_message(pk2id(&pub_key)),
        ))
        .await?;

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
