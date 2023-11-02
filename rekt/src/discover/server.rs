use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use tokio::net::UdpSocket;
use tokio::time::interval;
use tokio_stream::StreamExt;

use crate::constants::{BOOTSTRAP_NODES, DEFAULT_PORT};
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::discover_node::DiscoverNode;
use super::messages::discover_message::{DiscoverMessage, DEFAULT_MESSAGE_EXPIRATION};

use super::messages::ping_pong_messages::PingMessage;

pub struct Server {
    local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    udp_receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,
    udp_sender: kanal::AsyncSender<(SocketAddr, Bytes)>,

    pub(super) nodes: DashMap<H512, DiscoverNode>,

    pub(super) pending_pings: DashMap<H512, std::time::Instant>,
}

impl Server {
    pub async fn new(local_node: LocalNode, nodes: Vec<String>) -> Result<Self, io::Error> {
        let udp_socket = Arc::new(
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                DEFAULT_PORT,
            )))
            .await?,
        );

        let nodes = DashMap::from_iter(
            nodes
                .into_iter()
                .filter_map(|n| n.parse::<NodeRecord>().ok())
                .filter_map(|n| DiscoverNode::try_from(n).ok())
                .map(|n| (n.node_record.id, n)),
        );

        let (sender, receiver) = kanal::unbounded_async();

        Ok(Self {
            local_node,
            udp_socket,
            nodes,
            udp_sender: sender,
            udp_receiver: receiver,
            pending_pings: DashMap::with_capacity(10_000),
        })
    }

    pub fn start(this: Arc<Self>) {
        let writer = this.clone();
        let reader = this.clone();
        let pinger = this.clone();
        let purger = this.clone();

        tokio::spawn(async move {
            let _ = writer.run_writer().await;
        });

        tokio::spawn(async move {
            let _ = reader.run_reader().await;
        });

        tokio::spawn(async move {
            let _ = pinger.run_pinger().await;
        });

        tokio::spawn(async move {
            let _ = purger.purge_stale_pings().await;
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
                    decode_msg_and_create_response(&self, &src, &buf[..size], &self.local_node.enr)
                {
                    let packet =
                        DiscoverMessage::create_disc_v4_packet(resp, &self.local_node.private_key);
                    let _ = self.udp_sender.send((src, packet)).await;
                }
            }
        }
    }

    //TODO: optimize this, we have await in each loop iteration
    //what we want is FuturesUnordered (or. smth like that)
    //the issue is borrowing node from DashMap
    //so to ping we could have smaller clonable struct
    async fn run_pinger(&self) -> anyhow::Result<()> {
        for n in self.nodes.iter() {
            self.send_ping_packet(&n).await;
        }

        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(DEFAULT_MESSAGE_EXPIRATION),
        ));

        while let Some(_) = stream.next().await {
            for n in self
                .nodes
                .iter()
                .filter(|n| n.should_ping(10 * DEFAULT_MESSAGE_EXPIRATION))
            {
                self.send_ping_packet(&n).await;
            }
        }

        Ok(())
    }

    async fn send_ping_packet(&self, node: &DiscoverNode) {
        if self.pending_pings.contains_key(&node.id()) {
            return;
        }

        if let Some(mut n) = self.nodes.get_mut(&node.id()) {
            if !n.should_ping(DEFAULT_MESSAGE_EXPIRATION) {
                return;
            }

            n.mark_ping_attempt();
        }

        self.pending_pings
            .insert(node.id(), std::time::Instant::now());
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

    async fn purge_stale_pings(&self) {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(DEFAULT_MESSAGE_EXPIRATION),
        ));

        while let Some(_) = stream.next().await {
            self.pending_pings
                .retain(|_, v| v.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION);
        }
    }
}
