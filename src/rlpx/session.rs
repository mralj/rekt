use futures::{SinkExt, StreamExt, TryStreamExt};
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info};

use crate::p2p::types::{P2PPeer, Protocol};
use crate::p2p::{self, HelloMessage};
use crate::p2p::{P2PMessage, P2PMessageID};
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::utils::pk2id;
use crate::rlpx::Connection;
use crate::types::message::{Message, MessageKind};

use crate::types::node_record::NodeRecord;

use super::errors::RLPXSessionError;
use super::io_connection::ConnectionIO;

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

        let (hello_msg, protocol_v) = match handle_hello_msg(&mut transport).await {
            Ok(hello_msg) => {
                info!("Received Hello: {:?}", hello_msg);
                let matched_protocol =
                    Protocol::match_protocols(&hello_msg.protocols, Protocol::get_our_protocols())
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

        // let mut peer = P2PPeer::new(node.str, transport, hello_msg.id, protocol_v);
        //
        // peer.run().await;

        let (w, r) = ConnectionIO::new(transport).split();
        let mut p = P2PPeer::new(node, hello_msg.id, protocol_v, r, w);
        p.read_messages().await
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
    Err(RLPXSessionError::ExpectedRLPXMessage)
}
