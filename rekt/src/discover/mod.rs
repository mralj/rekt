pub mod decoder;

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::net::UdpSocket;

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::{decode_msg_type, packet_size_is_valid};

pub async fn run_udp() -> Result<(), io::Error> {
    let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_PORT,
    )))
    .await?;
    println!("UDP listening on {}", socket.local_addr()?);

    let mut buf = vec![0u8; 1280];
    loop {
        // Receive data into the buffer. This will wait until data is sent to the specified address.
        let req = socket.recv_from(&mut buf).await;
        match req {
            Ok((size, _src)) => {
                if !packet_size_is_valid(size) {
                    continue;
                }

                decode_msg_type(&buf[..size]);
            }
            Err(e) => {
                println!("failed to receive from socket; err = {:?}", e);
            }
        }
    }
}
