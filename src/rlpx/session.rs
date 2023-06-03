use bytes::BytesMut;
use futures::{SinkExt, TryStreamExt};
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info, trace};

use crate::p2p::types::Capability;
use crate::p2p::{self, HelloMessage};
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

        match handle_hello_msg(&mut transport).await {
            Ok(hello_msg) => {
                info!("Received Hello: {:?}", hello_msg);
                match Capability::match_capabilities(
                    &hello_msg.capabilities,
                    Capability::get_our_capabilities(),
                ) {
                    Some(c) => {
                        info!("Matched capabilities: {:?}", c);
                    }
                    None => {
                        return Err(RLPXSessionError::NoMatchingCapabilities);
                    }
                }
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
        }

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
            if let Some(MessageKind::P2P(P2PMessage::Disconnect(reason))) = msg_kind {
                return Err(RLPXSessionError::DisconnectRequested(reason.to_owned()));
            }
        }

        return Err(RLPXSessionError::UnexpectedP2PMessage {
            received: msg_id,
            expected: P2PMessageID::Hello as u8,
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
    let msg_kind = msg.decode_kind()?;

    match msg_kind {
        None => Err(RLPXSessionError::UnknownError),
        Some(MessageKind::ETH) => {
            info!("Got ETH message with ID: {:?}", msg_id);
            Ok(())
        }
        Some(MessageKind::P2P(p2p_msg)) => {
            trace!("Got P2P msg: {:?}", p2p_msg);
            Ok(())
        }
    }
}
