use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use kanal::{AsyncReceiver, AsyncSender};
use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;

#[derive(Clone)]
pub struct DiscoveryServer {
    local_node: LocalNode,
    packet_rx: AsyncReceiver<(SocketAddr, DiscoverMessage)>,
    packet_tx: AsyncSender<(SocketAddr, DiscoverMessage)>,
}

impl DiscoveryServer {
    pub fn new(local_node: LocalNode) -> Self {
        let (packet_tx, packet_rx) = kanal::unbounded_async();
        Self {
            local_node,
            packet_rx,
            packet_tx,
        }
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
        let socket = match UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))
        .await
        {
            Ok(socket) => socket,
            Err(e) => {
                println!("Error binding socket: {}", e);
                return Err(e);
            }
        };

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let packet = socket.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }

                let response = decode_msg_and_create_response(&buf[..size], &self.local_node.enr);
                if response.is_none() {
                    continue;
                }

                let _ = self.packet_tx.send((src, response.unwrap())).await;
            }
        }
    }

    async fn run_writer(&self) -> Result<(), io::Error> {
        let socket = match UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))
        .await
        {
            Ok(socket) => socket,
            Err(e) => {
                println!("Error binding socket: {}", e);
                return Err(e);
            }
        };
        loop {
            if let Ok((sender, msg)) = self.packet_rx.recv().await {
                match socket
                    .send_to(
                        &DiscoverMessage::create_disc_v4_packet(msg, &self.local_node.private_key)
                            [..],
                        sender,
                    )
                    .await
                {
                    Ok(size) => println!("Sent {} bytes to {}", size, sender),
                    Err(e) => println!("Error sending packet: {}", e),
                }
            }
        }
    }
}
