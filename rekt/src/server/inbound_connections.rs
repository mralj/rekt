use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use futures::SinkExt;
use secp256k1::SecretKey;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpSocket,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Decoder;

use crate::{
    constants::DEFAULT_PORT,
    rlpx::{Connection, RLPXMsg},
};

pub async fn run_incoming_connection_listener(our_secret_key: SecretKey) -> Result<(), io::Error> {
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseport(true)?;
    socket.set_reuseaddr(true)?;
    socket.bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_PORT,
    )))?;
    println!("TCP Server listening on {}", socket.local_addr()?);

    let listener = socket.listen(1024)?;
    loop {
        let (stream, addr) = listener.accept().await?;

        tokio::spawn(async move {
            println!("Accepted connection from {}", addr);
            let rlpx_connection = Connection::new_in(our_secret_key);
            let mut transport = rlpx_connection.framed(stream);
            if let Ok(Some(msg)) = transport.try_next().await {
                if matches!(msg, RLPXMsg::Auth) {
                    println!("Received auth message from {}", addr);
                    if let Err(e) = transport.send(crate::rlpx::codec::RLPXMsgOut::Ack).await {
                        println!("Failed to send ack message to {}: {}", addr, e);
                    }
                } else {
                    println!("Received unexpected message from {}", addr);
                }
            }
        });
    }
}
