use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use kanal::{AsyncReceiver, AsyncSender};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, UdpSocket};

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;

#[derive(Clone)]
pub struct DiscoveryServer {
    local_node: LocalNode,

    socket_rx: Arc<UdpSocket>,
    socket_tx: Arc<UdpSocket>,

    packet_rx: AsyncReceiver<(SocketAddr, Bytes, Instant)>,
    packet_tx: AsyncSender<(SocketAddr, Bytes, Instant)>,
}

impl DiscoveryServer {
    pub async fn new(local_node: LocalNode) -> Result<Self, io::Error> {
        let (packet_tx, packet_rx) = kanal::unbounded_async();
        let socket = Arc::new(
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                DEFAULT_PORT,
            )))
            .await?,
        );

        Ok(Self {
            local_node,

            socket_rx: Arc::clone(&socket),
            socket_tx: Arc::clone(&socket),

            packet_rx,
            packet_tx,
        })
    }

    pub async fn start(&self) {
        let reader = self.clone();
        let writer = self.clone();

        tokio::task::spawn(async move {
            let _ = reader.run_reader().await;
        });
        tokio::task::spawn(async move {
            let _ = writer.run_writer().await;
        });
    }

    async fn run_reader(&self) -> Result<(), io::Error> {
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let packet = self.socket_rx.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }

                let _ = self
                    .packet_tx
                    .send((src, Bytes::copy_from_slice(&buf[..size]), Instant::now()))
                    .await;
            }
        }
    }

    async fn run_writer(&self) -> Result<(), io::Error> {
        loop {
            if let Ok((sender, buf, _)) = self.packet_rx.recv().await {
                let response =
                    decode_msg_and_create_response(&buf[..], &self.local_node.enr, &sender);
                if response.is_none() {
                    continue;
                }

                let _ = self
                    .socket_tx
                    .send_to(
                        &DiscoverMessage::create_disc_v4_packet(
                            response.unwrap(),
                            &self.local_node.private_key,
                        )[..],
                        sender,
                    )
                    .await;
            }
        }
    }
}

pub async fn run_tcp() -> Result<(), io::Error> {
    let socket = match TcpSocket::new_v4() {
        Ok(socket) => socket,
        Err(e) => {
            println!("Failed to create socket: {:?}", e);
            return Err(e);
        }
    };

    match socket.set_reuseport(true) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to set reuseport: {:?}", e);
            return Err(e);
        }
    }
    match socket.set_reuseaddr(true) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to set reuse addr: {:?}", e);
            return Err(e);
        }
    }

    match socket.bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_PORT,
    ))) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to bind socket: {:?}", e);
            return Err(e);
        }
    }
    println!("TCP Server listening on {}", socket.local_addr()?);

    let listener = socket.listen(1024)?;
    loop {
        let (mut socket, addr) = listener.accept().await?;

        if addr.port() == 30311 {
            println!("Accepted connection from {}", addr);
        }

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    // Return or break depending on your application's needs
                    Ok(n) if n == 0 => return, // EOF
                    Ok(n) => {
                        // Echo back to the client
                        if let Err(e) = socket.write_all(&buf[..n]).await {
                            eprintln!("Failed to write to socket: {}", e);
                            return;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from socket: {}", e);
                        return;
                    }
                }
            }
        });
    }
}
