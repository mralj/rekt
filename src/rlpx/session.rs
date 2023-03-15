use futures::{SinkExt, TryStreamExt};
use secp256k1::SecretKey;
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::rlpx::codec::RLPXMsg;
use crate::rlpx::errors::RLPXError;
use crate::rlpx::Connection;
use crate::types::node_record::NodeRecord;

use super::errors::RLPXSessionError;

pub fn connect_to_node(
    node: NodeRecord,
    secret_key: SecretKey,
) -> tokio::task::JoinHandle<Result<(), RLPXSessionError>> {
    tokio::spawn(async move {
        let rlpx_connection = Connection::new(secret_key, node.pub_key);
        let stream = TcpStream::connect(node.get_socket_addr()).await?;

        let mut transport = rlpx_connection.framed(stream);
        transport.send(RLPXMsg::Auth).await?;

        trace!("waiting for RLPX ack ...");
        let msg = transport.try_next().await?;
        let msg = msg.ok_or(RLPXError::InvalidAckData)?;

        if matches!(msg, RLPXMsg::Ack) {
            trace!("Got RLPX ack");
        } else {
            trace!("Got unexpected message: {:?}", msg);
            return Err(RLPXSessionError::UnexpectedMessage {
                received: msg,
                expected: RLPXMsg::Ack,
            });
        }

        loop {
            match transport.try_next().await {
                Err(e) => {
                    eprintln!("Failed to receive message: {}", e);
                    return Ok(());
                }
                Ok(Some(msg)) => {
                    trace!("Got message: {:?}", msg);
                }
                _ => {}
            }
        }
    })
}
