use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
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
            if let Ok((sender, buf, start)) = self.packet_rx.recv().await {
                let response = decode_msg_and_create_response(&buf[..], &self.local_node.enr);
                if response.is_none() {
                    continue;
                }

                self.socket_tx
                    .send_to(
                        &DiscoverMessage::create_disc_v4_packet(
                            response.unwrap(),
                            &self.local_node.private_key,
                        )[..],
                        sender,
                    )
                    .await?;

                println!("Time elapsed: {:?}", start.elapsed());
            }
        }
    }
}
