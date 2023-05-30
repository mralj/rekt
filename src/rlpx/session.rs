use bytes::BytesMut;
use futures::{SinkExt, TryStreamExt};
use open_fastrlp::Decodable;
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info, trace};

use crate::p2p;
use crate::p2p::{MessageID, P2PMessageID};
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::utils::pk2id;
use crate::rlpx::Connection;

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
                return Err(RLPXSessionError::UnknownError);
            }

            match msg.unwrap() {
                RLPXMsg::Message(m) => handle_messages(m)?,
                _ => {
                    return Err(RLPXSessionError::UnknownError);
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

    match maybe_rlpx_msg {
        RLPXMsg::Message(rlpx_msg) => {
            let msg_id = p2p::P2PMessageID::decode(&mut &rlpx_msg[..])?;
            if msg_id != p2p::P2PMessageID::Hello {
                error!("ID: Got unexpected message: {:?}", msg_id);
                return Err(RLPXSessionError::UnknownError);
            }

            let msg = p2p::P2PMessage::decode(msg_id, &mut &rlpx_msg[1..])
                .map_err(|e| RLPXError::DecodeError(e.to_string()))?;

            match msg {
                p2p::P2PMessage::Hello(node_info) => {
                    info!("Received Hello: {:?}", node_info);
                    Ok(())
                }
                _ => {
                    error!("MSG: Got unexpected message: {:?}", msg);
                    Err(RLPXSessionError::UnknownError)
                }
            }
        }
        _ => {
            error!("Not RLPX: Got unexpected message: {:?}", maybe_rlpx_msg);
            Err(RLPXSessionError::UnexpectedMessage {
                received: maybe_rlpx_msg,
                expected: RLPXMsg::Message(BytesMut::new()),
            })
        }
    }
}

fn handle_messages(bytes: BytesMut) -> Result<(), RLPXSessionError> {
    let msg_id = MessageID::decode(&mut &bytes[..])?;

    match msg_id {
        MessageID::P2PMessageID(P2PMessageID::Ping) => {
            trace!("Got ping request")
        }
        MessageID::P2PMessageID(P2PMessageID::Pong) => {
            trace!("Got pong request")
        }
        MessageID::P2PMessageID(P2PMessageID::Hello) => {
            trace!("Got hello")
        }
        MessageID::P2PMessageID(P2PMessageID::Disconnect) => {
            let p2p_msg = p2p::P2PMessage::decode(P2PMessageID::Disconnect, &mut &bytes[1..])?;
            error!("Got Disconnect: {:?}", p2p_msg)
        }
        MessageID::CapabilityMessageId(id) => {
            info!("Got CAP message: {:?}", id)
        }
    }

    Ok(())
}
