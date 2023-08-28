use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

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
        if let Ok((size, src)) = packet {
            if !packet_size_is_valid(size) {
                continue;
            }

            let mut test = false;
            if src.ip() == IpAddr::V4(Ipv4Addr::new(109, 60, 95, 182)) {
                test = true;
            }

            let response = decode_msg(&buf[..size], test);
            if response.is_some() {
                if test {
                    println!("Sending pong to {:?}", src);
                }

                if let Err(e) = socket.send_to(&response.unwrap()[..], src).await {
                    println!("Error sending pong {:?}", e);
                }
            }
        }
    }
}
