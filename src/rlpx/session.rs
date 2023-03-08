use futures::{SinkExt, TryStreamExt};
use secp256k1::SecretKey;
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::rlpx::Connection;
use crate::types::node_record::NodeRecord;

pub fn connect_to_node(node: NodeRecord, secret_key: SecretKey) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let rlpx_connection = Connection::new(secret_key, node.pub_key);
        let stream = match TcpStream::connect(node.get_socket_addr()).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                return;
            }
        };

        let mut transport = rlpx_connection.framed(stream);
        match transport.send(super::codec::RLPXMsg::Auth).await {
            Ok(_) => {
                trace!("Sent auth")
            }
            Err(e) => {
                eprintln!("Failed to send auth: {}", e);
                return;
            }
        }

        trace!("waiting for RLPX ack ...");
        let msg = transport.try_next().await.unwrap();
        let msg = match msg.ok_or(super::errors::RLPXError::InvalidAckData) {
            Ok(msg) => msg,
            Err(e) => {
                trace!("Failed to decode ack: {}", e);
                return;
            }
        };

        if msg == super::codec::RLPXMsg::Ack {
            trace!("Got RLPX ack");
        } else {
            trace!("Got unexpected message: {:?}", msg);
        }

        loop {
            match transport.try_next().await {
                Err(e) => {
                    eprintln!("Failed to receive message: {}", e);
                    return;
                }
                Ok(Some(msg)) => {
                    trace!("Got message: {:?}", msg);
                }
                _ => {}
            }
        }
    })
}
