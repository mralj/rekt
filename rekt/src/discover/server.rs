use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use bytes::Bytes;
use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;

pub struct Server {
    local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,
    sender: kanal::AsyncSender<(SocketAddr, Bytes)>,
}

impl Server {
    pub async fn new(local_node: LocalNode) -> Result<Self, io::Error> {
        let udp_socket = Arc::new(
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                DEFAULT_PORT,
            )))
            .await?,
        );

        let (sender, receiver) = kanal::unbounded_async();

        Ok(Self {
            local_node,
            udp_socket,
            sender,
            receiver,
        })
    }

    pub fn start(this: Arc<Self>) {
        let writer = this.clone();
        let reader = this.clone();

        tokio::spawn(async move {
            let _ = writer.run_writer().await;
        });

        tokio::spawn(async move {
            let _ = reader.run_reader().await;
        });
    }

    async fn run_writer(&self) -> Result<(), io::Error> {
        let udp_socket = self.udp_socket.clone();

        loop {
            if let Ok((dest, packet)) = self.receiver.recv().await {
                let _ = udp_socket.send_to(&packet, dest).await;
            }
        }
    }

    async fn run_reader(&self) -> Result<(), io::Error> {
        let socket = self.udp_socket.clone();
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let packet = socket.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }

                if let Some(resp) =
                    decode_msg_and_create_response(&buf[..size], &self.local_node.enr)
                {
                    let packet =
                        DiscoverMessage::create_disc_v4_packet(resp, &self.local_node.private_key);
                    let _ = self.sender.send((src, packet)).await;
                }
            }
        }
    }
}
