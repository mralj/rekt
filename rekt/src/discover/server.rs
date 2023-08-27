use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::{decode_msg, packet_size_is_valid};

use super::decoder::MAX_PACKET_SIZE;

pub async fn run_discovery_server() -> Result<(), io::Error> {
    let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_PORT,
    )))
    .await?;

    let mut buf = vec![0u8; MAX_PACKET_SIZE];
    loop {
        let packet = socket.recv_from(&mut buf).await;
        if let Ok((size, _src)) = packet {
            if !packet_size_is_valid(size) {
                continue;
            }

            decode_msg(&buf[..size]);
        }
    }
}
