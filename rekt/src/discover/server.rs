use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use secp256k1::SecretKey;
use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::{decode_msg, packet_size_is_valid};

use super::decoder::MAX_PACKET_SIZE;
use super::messages::discover_message::DiscoverMessage;

pub async fn run_discovery_server(secret_key: &SecretKey) -> Result<(), io::Error> {
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

            let response = decode_msg(&buf[..size]);
            if response.is_some() {
                let _ = socket
                    .send_to(
                        &DiscoverMessage::create_disc_v4_packet(response.unwrap(), secret_key)[..],
                        src,
                    )
                    .await;
            }
        }
    }
}
