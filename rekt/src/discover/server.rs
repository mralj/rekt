use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;

use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;
use crate::types::node_record::NodeRecord;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;

pub struct Server {
    local_node: LocalNode,
    nodes: Vec<NodeRecord>,
}

impl Server {
    pub fn new(local_node: LocalNode, nodes: Vec<String>) -> Self {
        let nodes = nodes
            .iter()
            .map(|n| n.as_str())
            .map(NodeRecord::from_str)
            .filter_map(Result::ok)
            .collect();

        Self { local_node, nodes }
    }

    pub async fn run(&self) -> Result<(), io::Error> {
        let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))
        .await?;

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
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
}
