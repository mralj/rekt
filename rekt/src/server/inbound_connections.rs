use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::atomic::AtomicBool,
    time::Duration,
};

use futures::{SinkExt, TryStreamExt};
use secp256k1::PublicKey;
use tokio::net::{TcpSocket, TcpStream};
use tokio_util::codec::{Decoder, Framed};
use tracing::error;

use crate::{
    constants::DEFAULT_PORT,
    local_node::LocalNode,
    rlpx::{Connection, RLPXError, RLPXMsg, RLPXSessionError},
};

pub struct InboundConnections {
    our_pub_key: PublicKey,
    our_private_key: secp256k1::SecretKey,

    is_paused: AtomicBool,
}

impl InboundConnections {
    pub fn new(local_node: LocalNode) -> Self {
        Self {
            our_pub_key: local_node.public_key,
            our_private_key: local_node.private_key,
            is_paused: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn stop_listener(&self) -> bool {
        self.is_paused
            .swap(true, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn start_listener(&self) -> bool {
        self.is_paused
            .swap(false, std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn run(&self) -> Result<(), io::Error> {
        let socket = TcpSocket::new_v4()?;
        socket.set_reuseport(true)?;
        socket.set_reuseaddr(true)?;
        socket.bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))?;
        println!("TCP Server listening on {}", socket.local_addr()?);

        let our_secret_key = self.our_private_key;
        let listener = socket.listen(1024)?;
        loop {
            let (stream, addr) = listener.accept().await?;
            if self.is_paused() {
                tokio::time::sleep(Duration::from_secs(120)).await;
                continue;
            }

            tokio::spawn(async move {
                let rlpx_connection = Connection::new_in(our_secret_key);
                let mut transport = rlpx_connection.framed(stream);
                if handle_auth_msg(&mut transport).await.is_err() {
                    return;
                } else {
                    println!("Authenticated connection from {}", addr);
                }
            });
        }
    }
}

async fn handle_auth_msg(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<(), RLPXSessionError> {
    let msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidAuthData)?;

    if !matches!(msg, RLPXMsg::Auth) {
        error!("Got unexpected message: {:?}", msg);
        return Err(RLPXSessionError::UnexpectedMessage {
            received: msg,
            expected: RLPXMsg::Auth,
        });
    }

    transport.send(crate::rlpx::codec::RLPXMsgOut::Ack).await?;
    Ok(())
}
