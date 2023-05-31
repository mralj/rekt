use bytes::BytesMut;
use futures::{SinkExt, TryStreamExt};
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info, trace};

use crate::p2p;
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::utils::pk2id;
use crate::rlpx::Connection;
use crate::types::message::{Message, MessageKind};

use crate::types::node_record::NodeRecord;

use super::errors::RLPXSessionError;

pub fn connect_to_node(
    node: NodeRecord,
    secret_key: SecretKey,
    pub_key: PublicKey,
) -> tokio::task::JoinHandle<Result<(), RLPXSessionError>> {
    tokio::spawn(async move {
        let rlpx_connection = Connection::new(secret_key, node.pub_key);
        let stream = TcpStream::connect(node.get_socket_addr()).await?;

        let mut transport = rlpx_connection.framed(stream);
        transport.send(RLPXMsg::Auth).await?;

        handle_ack_msg(&mut transport).await?;

        transport
            .send(RLPXMsg::Message(
                p2p::HelloMessage::make_our_hello_message(pk2id(&pub_key)).rlp_encode(),
            ))
            .await?;

        handle_hello_msg(&mut transport).await?;

        loop {
            let msg = transport.try_next().await?;
            if msg.is_none() {
                return Err(RLPXSessionError::NoMessage);
            }

            let msg = msg.unwrap();
            match msg {
                RLPXMsg::Message(m) => handle_messages(m)?,
                _ => {
                    return Err(RLPXSessionError::ExpectedRLPXMessage { received: msg });
                }
            }
        }
    })
}

async fn handle_ack_msg(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<(), RLPXSessionError> {
    trace!("waiting for RLPX ack ...");
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
) -> Result<(), RLPXSessionError> {
    let maybe_rlpx_msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidMsgData)?;

    if let RLPXMsg::Message(rlpx_msg) = maybe_rlpx_msg {
        let mut msg = Message::new(rlpx_msg);
        let msg_id = msg.decode_id()?;
        if msg_id != P2PMessageID::Hello as u8 {
            error!("ID: Got unexpected message: {:?}", msg_id);
            return Err(RLPXSessionError::UnexpectedMessageID {
                received: msg_id,
                expected: P2PMessageID::Hello,
            });
        }

        msg.decode_kind()?;
        if let MessageKind::P2P(P2PMessage::Hello(node_info)) = msg.kind {
            info!("Received Hello: {:?}", node_info);
            return Ok(());
        }

        error!("MSG: Got unexpected message: {:?}", msg);
        return Err(RLPXSessionError::UnexpectedP2PMessage {
            received: msg.kind,
            expected: MessageKind::P2P(P2PMessage::Hello(p2p::HelloMessage::empty())),
        });
    }

    error!("Not RLPX: Got unexpected message: {:?}", maybe_rlpx_msg);
    Err(RLPXSessionError::ExpectedRLPXMessage {
        received: maybe_rlpx_msg,
    })
}

fn handle_messages(bytes: BytesMut) -> Result<(), RLPXSessionError> {
    let mut msg = Message::new(bytes);
    let msg_id = msg.decode_id()?;
    msg.decode_kind()?;

    match msg.kind {
        MessageKind::Unknown => Err(RLPXSessionError::UnknownError),
        MessageKind::ETH => {
            info!("Got ETH message with ID: {:?}", msg_id);
            Ok(())
        }
        MessageKind::P2P(p2p_msg) => {
            trace!("Got P2P msg: {:?}", p2p_msg);
            Ok(())
        }
    }
}
