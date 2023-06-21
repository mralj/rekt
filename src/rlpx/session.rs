use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, TryStreamExt};
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info};

use crate::p2p::types::p2p_wire::P2PWire;
use crate::p2p::types::{P2PPeer, Protocol};
use crate::p2p::{self, HelloMessage};
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::utils::pk2id;
use crate::rlpx::Connection;
use crate::types::hash::H512;
use crate::types::message::{Message, MessageKind};

use crate::types::node_record::NodeRecord;

use super::errors::RLPXSessionError;
use super::tcp_transport::TcpTransport;

pub fn connect_to_node(
    node: NodeRecord,
    secret_key: SecretKey,
    pub_key: PublicKey,
    semaphore: Arc<Semaphore>,
    peers: Arc<Mutex<HashMap<H512, String>>>,
) -> tokio::task::JoinHandle<Result<(), RLPXSessionError>> {
    tokio::spawn(async move {
        let permit = semaphore
            .acquire()
            .await
            .expect("Semaphore should've been live");

        let rlpx_connection = Connection::new(secret_key, node.pub_key);
        let stream = match timeout(
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
        }?;

        let mut transport = rlpx_connection.framed(stream);
        transport.send(RLPXMsg::Auth).await?;

        handle_ack_msg(&mut transport).await?;

        transport
            .send(RLPXMsg::Message(p2p::HelloMessage::get_our_hello_message(
                pk2id(&pub_key),
            )))
            .await?;

        let (hello_msg, protocol_v) = match handle_hello_msg(&mut transport).await {
            Ok(mut hello_msg) => {
                info!("Received Hello: {:?}", hello_msg);
                let matched_protocol = Protocol::match_protocols(&mut hello_msg.protocols)
                    .ok_or(RLPXSessionError::NoMatchingProtocols)?;

                (hello_msg, matched_protocol.version)
            }
            Err(e) => {
                if let RLPXSessionError::DisconnectRequested(reason) = e {
                    //NOTE: we can further handle disconnects here
                    // like logging this to file or deciding to retry based on disconnect
                    // reason/count
                    error!("Disconnect requested: {}", reason);
                }
                return Err(e);
            }
        };

        // releases semaphore permit, so that concurrent connection attempts can proceed
        drop(permit);

        let mut p = P2PPeer::new(
            node,
            hello_msg.id,
            protocol_v,
            hello_msg.client_version,
            P2PWire::new(TcpTransport::new(transport)),
        );

        let task_result = p.run(peers.clone()).await;

        {
            peers.lock().unwrap().remove(&p.id);
        }

        task_result
    })
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
    let maybe_rlpx_msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidMsgData)?;

    if let RLPXMsg::Message(rlpx_msg) = maybe_rlpx_msg {
        let mut msg = Message::new(rlpx_msg);
        let msg_id = msg.decode_id()?;

        if msg_id == P2PMessageID::Hello as u8 {
            msg.decode_kind()?;
            if let Some(MessageKind::P2P(P2PMessage::Hello(node_info))) = msg.kind {
                return Ok(node_info);
            }
        }

        if msg_id == P2PMessageID::Disconnect as u8 {
            let msg_kind = msg.decode_kind()?;
            if let MessageKind::P2P(P2PMessage::Disconnect(reason)) = msg_kind {
                return Err(RLPXSessionError::DisconnectRequested(reason.to_owned()));
            }
        }

        return Err(RLPXSessionError::UnexpectedP2PMessage {
            received: msg_id,
            expected: P2PMessageID::Hello as u8,
        });
    }

    error!("Not RLPX: Got unexpected message: {:?}", maybe_rlpx_msg);
    Err(RLPXSessionError::ExpectedRLPXMessage)
}
