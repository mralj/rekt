use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use tokio::net::UdpSocket;

use crate::constants::{BOOTSTRAP_NODES, DEFAULT_PORT};
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;
use super::messages::enr::EnrRequest;
use super::messages::find_node::FindNode;
use super::messages::ping_pong_messages::PingMessage;

pub struct Server {
    local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,
    sender: kanal::AsyncSender<(SocketAddr, Bytes)>,

    boot_nodes: Vec<NodeRecord>,
    static_nodes: Vec<String>,
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
        let boot_nodes: Vec<NodeRecord> = BOOTSTRAP_NODES
            .iter()
            .copied()
            .filter_map(|n| NodeRecord::from_str(n).ok())
            .collect();

        Ok(Self {
            local_node,
            udp_socket,
            sender,
            receiver,
            boot_nodes,
            static_nodes: Vec::new(),
        })
    }

    pub fn start(this: Arc<Self>) {
        let writer = this.clone();
        let reader = this.clone();
        let lookup: Arc<Server> = this.clone();

        tokio::spawn(async move {
            let _ = writer.run_writer().await;
        });

        tokio::spawn(async move {
            let _ = reader.run_reader().await;
        });

        tokio::spawn(async move {
            let _ = lookup.run_lookup().await;
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
                    decode_msg_and_create_response(&src, &buf[..size], &self.local_node.enr)
                {
                    let packet =
                        DiscoverMessage::create_disc_v4_packet(resp, &self.local_node.private_key);
                    let _ = self.sender.send((src, packet)).await;
                }
            }
        }
    }

    async fn run_lookup(&self) {
        for boot_node in &self.boot_nodes {
            if let IpAddr::V4(address) = boot_node.address {
                let _ = self
                    .sender
                    .send((
                        SocketAddr::V4(SocketAddrV4::new(address, boot_node.tcp_port)),
                        DiscoverMessage::create_disc_v4_packet(
                            DiscoverMessage::Ping(PingMessage::new(&self.local_node, boot_node)),
                            &self.local_node.private_key,
                        ),
                    ))
                    .await;
            }
        }

        //TODO: implement this properly later on, the timer is here just to make sure that
        // ping messages are sent before the lookup is started
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        for boot_node in &self.boot_nodes {
            if let IpAddr::V4(address) = boot_node.address {
                let _ = self
                    .sender
                    .send((
                        SocketAddr::V4(SocketAddrV4::new(address, boot_node.tcp_port)),
                        DiscoverMessage::create_disc_v4_packet(
                            DiscoverMessage::EnrRequest(EnrRequest::new()),
                            &self.local_node.private_key,
                        ),
                    ))
                    .await;
            }
        }
    }
}
