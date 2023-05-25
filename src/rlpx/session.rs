use bytes::BytesMut;
use futures::{SinkExt, StreamExt, TryStreamExt};
use open_fastrlp::Decodable;
use secp256k1::{PublicKey, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{error, info, trace};

use crate::p2p;
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

        transport
            .for_each(|msg| {
                trace!("Got message: {:?}", msg);
                futures::future::ready(())
            })
            .await;

        Ok(())
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
    let msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidMsgData)?;

    match msg {
        RLPXMsg::Message(msg) => {
            let hello = p2p::P2PMessage::decode(&mut &msg[..])
                .map_err(|e| RLPXError::DecodeError(e.to_string()))?;

            info!("Got hello message: {:?}", hello);
            Ok(())
        }
        _ => {
            error!("Got unexpected message: {:?}", msg);
            Err(RLPXSessionError::UnexpectedMessage {
                received: msg,
                expected: RLPXMsg::Message(BytesMut::new()),
            })
        }
    }
}
