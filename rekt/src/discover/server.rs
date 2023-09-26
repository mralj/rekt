use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

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
}

impl Server {
    pub async fn new(local_node: LocalNode, nodes: Vec<String>) -> Result<Self, io::Error> {
        let nodes = nodes
            .iter()
            .map(|n| n.as_str())
            .map(NodeRecord::from_str)
            .filter_map(Result::ok)
            .collect();

        let udp_socket = Arc::new(
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                DEFAULT_PORT,
            )))
            .await?,
        );

        Ok(Self {
            local_node,
            nodes,
            udp_socket,
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
        let socket = self.udp_socket.clone();

        loop {
            let packet = socket.recv_from(&mut buf).await;
            if let Ok((size, src)) = packet {
                if !packet_size_is_valid(size) {
                    continue;
                }
                if let Some(response) =
                    decode_msg_and_create_response(&buf[..size], &self.local_node.enr, &src)
                {
                    let _ = socket
                        .send_to(
                            &DiscoverMessage::create_disc_v4_packet(
                                response,
                                &self.local_node.private_key,
                            )[..],
                            src,
                        )
                        .await;
                }
            }
        }
    }

    async fn run_pinger(&self) -> Result<(), io::Error> {
        let socket = self.udp_socket.clone();
        for node in &self.nodes {
            if let IpAddr::V4(address) = node.address {
                match socket
                    .send_to(
                        &DiscoverMessage::create_disc_v4_packet(
                            DiscoverMessage::Ping(PingMessage::new(&self.local_node, node)),
                            &self.local_node.private_key,
                        )[..],
                        SocketAddr::V4(SocketAddrV4::new(address, node.udp_port)),
                    )
                    .await
                {
                    Ok(_) => println!("Sent ping to {}", node.ip),
                    Err(e) => println!("Failed to send ping to {}: {:?}", node.ip, e),
                }
            }
        }

        Ok(())
    }
}
