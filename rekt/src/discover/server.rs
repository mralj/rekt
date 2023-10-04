use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use dashmap::{DashMap, DashSet};
use tokio::net::UdpSocket;

use crate::constants::{BOOTSTRAP_NODES, DEFAULT_PORT};
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::discover_node::DiscoverNode;
use super::messages::discover_message::DiscoverMessage;
use super::messages::enr::EnrRequest;
use super::messages::find_node::FindNode;
use super::messages::ping_pong_messages::PingMessage;

pub struct Server {
    local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    udp_receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,
    udp_sender: kanal::AsyncSender<(SocketAddr, Bytes)>,

    nodes: DashMap<H512, DiscoverNode>,

    pending_pings: DashSet<H512>,

    //TODO delete this
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
            udp_sender: sender,
            udp_receiver: receiver,
            boot_nodes,
            static_nodes: Vec::new(),
            nodes: DashMap::with_capacity(10_0000),
            pending_pings: DashSet::with_capacity(10_000),
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
            if let Ok((dest, packet)) = self.udp_receiver.recv().await {
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
                    let _ = self.udp_sender.send((src, packet)).await;
                }
            }
        }
    }

    async fn send_ping_packet(&self, node: &DiscoverNode) {
        if self.pending_pings.contains(&node.id()) {
            return;
        }

        if let Some(mut n) = self.nodes.get_mut(&node.id()) {
            if n.re_ping_is_not_needed() {
                return;
            }

            n.mark_ping_attempt();
        }

        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::Ping(PingMessage::new(&self.local_node, &node.node_record)),
            &self.local_node.private_key,
        );

        let _ = self
            .udp_sender
            .send((
                SocketAddr::V4(SocketAddrV4::new(node.ip_v4_addr, node.udp_port())),
                packet,
            ))
            .await;
    }
}
