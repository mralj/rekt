use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;
use super::messages::ping_pong_messages::PingMessage;

#[derive(Debug, Clone)]
pub struct Server {
    local_node: LocalNode,
    nodes: Vec<NodeRecord>,

    udp_socket: Arc<UdpSocket>,
    receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,
    sender: kanal::AsyncSender<(SocketAddr, Bytes)>,
}

impl Server {
    pub async fn new(local_node: LocalNode, nodes: Vec<String>) -> Result<Self, io::Error> {
        let nodes = nodes
            .iter()
            .map(|n| n.as_str())
            .map(NodeRecord::from_str)
            .filter_map(Result::ok)
            .collect();

        let udp_socket = match UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))
        .await
        {
            Ok(s) => s,
            Err(e) => {
                println!("Failed to bind udp socket: {:?}", e);
                return Err(e);
            }
        };

        let udp_socket = Arc::new(udp_socket);
        let (sender, receiver) = kanal::unbounded_async();

        Ok(Self {
            local_node,
            nodes,
            udp_socket,
            sender,
            receiver,
        })
    }

    pub async fn new_arc(
        local_node: LocalNode,
        nodes: Vec<String>,
    ) -> Result<Arc<Self>, io::Error> {
        Ok(Arc::new(Self::new(local_node, nodes).await?))
    }

    pub async fn start(server: Arc<Self>) -> Result<(), io::Error> {
        let pinger = server.clone();
        let listener = server.clone();
        let sender = server.clone();

        tokio::task::spawn(async move {
            let _ = sender.run_sender().await;
        });

        tokio::task::spawn(async move {
            let _ = listener.run_listener().await;
        });

        tokio::task::spawn(async move {
            let _ = pinger.run_pinger().await;
        });

        Ok(())
    }

    async fn run_listener(&self) -> Result<(), io::Error> {
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        let udp_socket = self.udp_socket.clone();

        loop {
            let packet = udp_socket.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }
                if let Some(response) =
                    decode_msg_and_create_response(&buf[..size], &self.local_node.enr, &src)
                {
                    let packet = DiscoverMessage::create_disc_v4_packet(
                        response,
                        &self.local_node.private_key,
                    );

                    let _ = self.sender.send((src, packet)).await;
                }
            }
        }
    }

    async fn run_sender(&self) -> Result<(), io::Error> {
        let udp_socket = self.udp_socket.clone();
        loop {
            if let Ok((socket_addr, packet)) = self.receiver.recv().await {
                let _ = udp_socket.send_to(packet.as_ref(), socket_addr).await;
            }
        }
    }

    async fn run_pinger(&self) -> Result<(), io::Error> {
        for node in &self.nodes {
            if let IpAddr::V4(address) = node.address {
                let packet = DiscoverMessage::create_disc_v4_packet(
                    DiscoverMessage::Ping(PingMessage::new(&self.local_node, node)),
                    &self.local_node.private_key,
                );

                let _ = &self
                    .sender
                    .send((
                        SocketAddr::V4(SocketAddrV4::new(address, node.udp_port)),
                        packet,
                    ))
                    .await;
            }
        }

        Ok(())
    }
}
