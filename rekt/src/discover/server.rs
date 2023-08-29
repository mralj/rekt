use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use secp256k1::SecretKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, UdpSocket};

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::packet_size_is_valid;
use crate::local_node::LocalNode;

use super::decoder::{decode_msg_and_create_response, MAX_PACKET_SIZE};
use super::messages::discover_message::DiscoverMessage;

pub async fn run_discovery_server(local_node: &LocalNode) -> Result<(), io::Error> {
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

            let response = decode_msg_and_create_response(&buf[..size], &local_node.enr);
            if response.is_none() {
                continue;
            }

            let _ = socket
                .send_to(
                    &DiscoverMessage::create_disc_v4_packet(
                        response.unwrap(),
                        &local_node.private_key,
                    )[..],
                    src,
                )
                .await;
        }
    }
}

pub async fn run_tcp() -> Result<(), io::Error> {
    let socket = match TcpSocket::new_v4() {
        Ok(socket) => socket,
        Err(e) => {
            println!("Failed to create socket: {:?}", e);
            return Err(e);
        }
    };

    match socket.set_reuseport(true) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to set reuseport: {:?}", e);
            return Err(e);
        }
    }
    match socket.set_reuseaddr(true) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to set reuse addr: {:?}", e);
            return Err(e);
        }
    }

    match socket.bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        DEFAULT_PORT,
    ))) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to bind socket: {:?}", e);
            return Err(e);
        }
    }
    println!("TCP Server listening on {}", socket.local_addr()?);

    let listener = socket.listen(1024)?;
    loop {
        let (mut socket, addr) = listener.accept().await?;

        println!("Accepted connection from {}", addr);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];

            loop {
                match socket.read(&mut buf).await {
                    // Return or break depending on your application's needs
                    Ok(n) if n == 0 => return, // EOF
                    Ok(n) => {
                        // Echo back to the client
                        if let Err(e) = socket.write_all(&buf[..n]).await {
                            eprintln!("Failed to write to socket: {}", e);
                            return;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from socket: {}", e);
                        return;
                    }
                }
            }
        });
    }
}
