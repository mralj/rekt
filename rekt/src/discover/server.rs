use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use kanal::AsyncSender;
use tokio::net::UdpSocket;
use tokio::time::interval;
use tokio_stream::StreamExt;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::discover::discover_node::{AuthStatus, DiscoverNodeType};
use crate::local_node::LocalNode;
use crate::server::connection_task::ConnectionTask;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::discover_node::DiscoverNode;
use super::messages::discover_message::{DiscoverMessage, DEFAULT_MESSAGE_EXPIRATION};

use super::messages::enr::EnrRequest;
use super::messages::find_node::{FindNode, Neighbours};
use super::messages::lookup::{Lookup, PendingNeighboursReq};
use super::messages::ping_pong_messages::PingMessage;

pub struct Server {
    pub(super) local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    pub(super) udp_sender: kanal::AsyncSender<(SocketAddr, Bytes)>,
    udp_receiver: kanal::AsyncReceiver<(SocketAddr, Bytes)>,

    pub(super) nodes: DashMap<H512, DiscoverNode>,

    pub(super) pending_pings: DashMap<H512, std::time::Instant>,

    pub(super) pending_neighbours_req: DashMap<H512, PendingNeighboursReq>,
    pub(super) pending_lookups: DashMap<H512, Lookup>,
    pub(super) conn_tx: AsyncSender<ConnectionTask>,
}

impl Server {
    pub async fn new(
        local_node: LocalNode,
        nodes: Vec<String>,
        conn_tx: AsyncSender<ConnectionTask>,
    ) -> Result<Self, io::Error> {
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
            conn_tx,
            pending_pings: DashMap::with_capacity(10_000),
            pending_neighbours_req: DashMap::with_capacity(100),
            pending_lookups: DashMap::with_capacity(100),
        })
    }

    pub fn start(this: Arc<Self>) {
        let writer = this.clone();
        let reader = this.clone();
        let worker = this.clone();
        let lookup = this.clone();
        let logger = this.clone();

        tokio::spawn(async move {
            let _ = writer.run_writer().await;
        });

        tokio::spawn(async move {
            let _ = Self::run_reader(reader).await;
        });

        tokio::spawn(async move {
            let _ = worker.run_worker().await;
        });

        tokio::spawn(async move {
            let _ = lookup.run_lookup().await;
        });

        tokio::spawn(async move {
            let _ = logger.run_logger().await;
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

    async fn run_reader(this: Arc<Self>) -> Result<(), io::Error> {
        let socket = this.udp_socket.clone();
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let packet = socket.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }

                if let Ok(msg) = decode_msg_and_create_response(src, &buf[..size]) {
                    let this = this.clone();
                    tokio::task::spawn(async move {
                        let _ = Self::handle_received_msg(this, msg).await;
                    });
                }
            }
        }
    }

    pub(super) async fn send_ping_packet(&self, node: (H512, NodeRecord, Ipv4Addr, u16)) {
        let (id, node_record, ip, udp) = node;
        if self.pending_pings.contains_key(&id) {
            return;
        }

        if let Some(mut n) = self.nodes.get_mut(&id) {
            if !n.should_ping(DEFAULT_MESSAGE_EXPIRATION) {
                return;
            }

            n.mark_ping_attempt();
        }

        self.pending_pings.insert(id, std::time::Instant::now());
        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::Ping(PingMessage::new(&self.local_node, &node_record)),
            &self.local_node.private_key,
        );

        let _ = self
            .udp_sender
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet))
            .await;
    }

    pub(super) async fn send_neighbours_packet(&self, lookup_id: H512, to: (Ipv4Addr, u16)) {
        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::FindNode(FindNode::new(lookup_id)),
            &self.local_node.private_key,
        );

        let (ip, udp) = to;
        let _ = self
            .udp_sender
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet))
            .await;
    }

    pub(super) async fn send_enr_req_packet(&self, to: (Ipv4Addr, u16)) {
        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::EnrRequest(EnrRequest::new()),
            &self.local_node.private_key,
        );

        let (ip, udp) = to;
        let _ = self
            .udp_sender
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet))
            .await;
    }

    async fn run_worker(&self) -> anyhow::Result<()> {
        let tasks = FuturesUnordered::from_iter(self.nodes.iter().map(|n| {
            self.send_ping_packet((n.id(), n.node_record.clone(), n.ip_v4_addr, n.udp_port()))
        }));

        let _result = tasks.collect::<Vec<_>>().await;

        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(DEFAULT_MESSAGE_EXPIRATION),
        ));

        while let Some(_) = stream.next().await {
            self.pending_pings
                .retain(|_, v| v.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION);

            self.pending_neighbours_req
                .retain(|_, v| v.created_on.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION);

            let tasks = FuturesUnordered::from_iter(
                self.nodes
                    .iter()
                    .filter(|n| n.should_ping(10 * DEFAULT_MESSAGE_EXPIRATION))
                    .map(|n| {
                        self.send_ping_packet((
                            n.id(),
                            n.node_record.clone(),
                            n.ip_v4_addr,
                            n.udp_port(),
                        ))
                    }),
            );
            let _result = tasks.collect::<Vec<_>>().await;

            let tasks = FuturesUnordered::from_iter(
                self.nodes
                    .iter()
                    .filter(|n| {
                        n.is_bsc_node.is_none()
                            && (n.auth_status() == AuthStatus::Authed
                                || n.auth_status() == AuthStatus::TheyAuthedUs)
                    })
                    .map(|n| self.send_enr_req_packet((n.ip_v4_addr, n.udp_port()))),
            );

            let _result = tasks.collect::<Vec<_>>().await;
        }

        Ok(())
    }

    async fn run_logger(&self) {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(60),
        ));

        while let Some(_) = stream.next().await {
            let mut len = 0;
            let mut we_auth = 0;
            let mut they_auth = 0;
            let mut not_authed = 0;
            let mut auth = 0;
            let mut conn_in = 0;
            let mut conn_out = 0;
            let mut bsc_nodes = 0;
            let mut non_bsc_nodes = 0;

            for n in self.nodes.iter() {
                len += 1;
                match n.auth_status() {
                    AuthStatus::Authed => {
                        auth += 1;
                    }
                    AuthStatus::TheyAuthedUs => {
                        they_auth += 1;
                    }
                    AuthStatus::WeAuthedThem => {
                        we_auth += 1;
                    }
                    AuthStatus::NotAuthed => {
                        not_authed += 1;
                    }
                }

                match n.node_type {
                    DiscoverNodeType::WeDiscoveredThem => {
                        conn_out += 1;
                    }
                    DiscoverNodeType::TheyDiscoveredUs => {
                        conn_in += 1;
                    }
                    _ => {}
                }

                if let Some(is_bsc_node) = n.is_bsc_node {
                    if is_bsc_node && n.node_type != DiscoverNodeType::Static {
                        bsc_nodes += 1;
                    } else if !is_bsc_node {
                        non_bsc_nodes += 1;
                    }
                }
            }
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

            println!("=== [{}] [DISC] ===\n Total: {len}, Authed: {auth}, They auth {they_auth}, We auth {we_auth}, No auth {not_authed}\n We discovered {conn_out}, They discovered {conn_in}\n BSC_NODES: {bsc_nodes}, no bsc: {non_bsc_nodes}", now);

            tracing::info!("=== [DISC] ===\n Total: {len}, Authed: {auth}, They auth {they_auth}, We auth {we_auth}, No auth {not_authed}\n We discovered {conn_out}, They discovered {conn_in}\n BSC_NODES: {bsc_nodes}, no bsc: {non_bsc_nodes}");
        }
    }
}
