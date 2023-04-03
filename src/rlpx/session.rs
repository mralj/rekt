use futures::{SinkExt, StreamExt, TryStreamExt};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::p2p;
use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::utils::pk2id;
use crate::rlpx::Connection;
use crate::types::hash::H512;
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

        trace!("waiting for RLPX ack ...");
        let msg = transport
            .try_next()
            .await?
            .ok_or(RLPXError::InvalidAckData)?;

        if !matches!(msg, RLPXMsg::Ack) {
            trace!("Got unexpected message: {:?}", msg);
            return Err(RLPXSessionError::UnexpectedMessage {
                received: msg,
                expected: RLPXMsg::Ack,
            });
        }

        transport
            .send(RLPXMsg::Message(
                p2p::HelloMessage::make_our_hello_message(pk2id(&pub_key)).rlp_encode(),
            ))
            .await?;

        transport
            .for_each(|msg| {
                trace!("Got message: {:?}", msg);
                futures::future::ready(())
            })
            .await;

        Ok(())
    })
}
