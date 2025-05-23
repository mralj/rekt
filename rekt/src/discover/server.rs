use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::time::interval;
use tokio_stream::StreamExt;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::discover::discover_node::AuthStatus;
use crate::local_node::LocalNode;
use crate::p2p::peer::is_buy_in_progress;
use crate::server::errors::ConnectionTaskError;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::discover_node::DiscoverNode;
use super::lookup::{Lookup, PendingNeighboursReq};
use super::messages::discover_message::{DiscoverMessage, DEFAULT_MESSAGE_EXPIRATION};

use super::messages::enr::EnrRequest;
use super::messages::find_node::FindNode;
use super::messages::ping_pong_messages::PingMessage;

pub struct Server {
    pub(super) local_node: LocalNode,
    udp_socket: Arc<UdpSocket>,

    pub(super) udp_sender: mpsc::UnboundedSender<(SocketAddr, Bytes)>,

    pub(super) nodes: DashMap<H512, DiscoverNode>,

    pub(super) pending_pings: DashMap<H512, std::time::Instant>,

    pub(super) pending_neighbours_req: DashMap<H512, PendingNeighboursReq>,
    pub(super) pending_lookups: DashMap<H512, Lookup>,

    pub(super) conn_tx: mpsc::UnboundedSender<ConnectionTaskError>,

    pub(super) is_paused: AtomicBool,

    pub(super) server_config: crate::cli::Cli,
}

impl Server {
    pub async fn new(
        local_node: LocalNode,
        nodes: Vec<String>,
        conn_tx: tokio::sync::mpsc::UnboundedSender<ConnectionTaskError>,
        udp_sender: mpsc::UnboundedSender<(SocketAddr, Bytes)>,
        server_config: crate::cli::Cli,
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

        Ok(Self {
            local_node,
            udp_socket,
            nodes,
            udp_sender,
            conn_tx,
            server_config,
            pending_pings: DashMap::with_capacity(10_000),
            pending_neighbours_req: DashMap::with_capacity(100),
            pending_lookups: DashMap::with_capacity(100),
            is_paused: AtomicBool::new(false),
        })
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn stop_disc_server(&self) {
        self.is_paused
            .swap(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn start_disc_server(&self) {
        self.is_paused
            .swap(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn start(this: Arc<Self>, udp_receiver: UnboundedReceiver<(SocketAddr, Bytes)>) {
        let writer = this.clone();
        let reader = this.clone();
        let worker = this.clone();

        tokio::spawn(async move {
            let _ = writer.run_writer(udp_receiver).await;
        });

        tokio::spawn(async move {
            let _ = reader.run_reader().await;
        });

        tokio::spawn(async move {
            let _ = worker.run_worker().await;
        });
    }

    async fn run_writer(
        &self,
        mut udp_receiver: UnboundedReceiver<(SocketAddr, Bytes)>,
    ) -> Result<(), io::Error> {
        let udp_socket = self.udp_socket.clone();

        loop {
            if let Some((dest, packet)) = udp_receiver.recv().await {
                if self.is_paused() {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    continue;
                }

                if is_buy_in_progress() {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    continue;
                }

                let _ = udp_socket.send_to(&packet, dest).await;
            }
        }
    }

    async fn run_reader(&self) -> Result<(), io::Error> {
        let socket = self.udp_socket.clone();
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let packet = socket.recv_from(&mut buf).await;
            if self.is_paused() {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                continue;
            }

            if is_buy_in_progress() {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                continue;
            }

            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }

                if let Ok(msg) = decode_msg_and_create_response(src, &buf[..size]) {
                    self.handle_received_msg(msg).await;
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
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet));
    }

    pub(super) async fn send_neighbours_packet(&self, lookup_id: H512, to: (Ipv4Addr, u16)) {
        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::FindNode(FindNode::new(lookup_id)),
            &self.local_node.private_key,
        );

        let (ip, udp) = to;
        let _ = self
            .udp_sender
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet));
    }

    pub(super) async fn send_enr_req_packet(&self, to: (Ipv4Addr, u16)) {
        let packet = DiscoverMessage::create_disc_v4_packet(
            DiscoverMessage::EnrRequest(EnrRequest::new()),
            &self.local_node.private_key,
        );

        let (ip, udp) = to;
        let _ = self
            .udp_sender
            .send((SocketAddr::V4(SocketAddrV4::new(ip, udp)), packet));
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
            if self.is_paused() {
                continue;
            }

            if is_buy_in_progress() {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                continue;
            }

            self.pending_pings
                .retain(|_, v| v.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION);

            self.pending_neighbours_req
                .retain(|_, v| v.created_on.elapsed().as_secs() < DEFAULT_MESSAGE_EXPIRATION);

            let pending_lookups_to_retain = self
                .pending_neighbours_req
                .iter()
                .map(|v| v.lookup_id)
                .collect::<Vec<_>>();

            self.pending_lookups
                .retain(|k, _| pending_lookups_to_retain.contains(k));

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

            let next_lookup_id = self.get_next_lookup_id();
            let closest_nodes = self.get_closest_nodes(next_lookup_id);
            self.pending_lookups.insert(
                next_lookup_id,
                Lookup::new(next_lookup_id, closest_nodes.clone()),
            );

            for n in closest_nodes.iter() {
                self.pending_neighbours_req
                    .insert(n.id(), PendingNeighboursReq::new(next_lookup_id, n));
            }

            let tasks = FuturesUnordered::from_iter(closest_nodes.iter().map(|n| {
                self.send_neighbours_packet(next_lookup_id, (n.ip_v4_addr, n.udp_port()))
            }));

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

    pub fn get_bsc_node_enodes(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|n| {
                if let Some(bsc_node) = n.is_bsc_node {
                    bsc_node
                } else {
                    false
                }
            })
            .map(|n| n.node_record.str.clone())
            .collect::<Vec<_>>()
    }
}
