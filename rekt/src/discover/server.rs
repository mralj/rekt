use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use secp256k1::SecretKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, UdpSocket};

use crate::constants::DEFAULT_PORT;
use crate::discover::decoder::{decode_msg, packet_size_is_valid};

use super::decoder::{create_disc_v4_packet, MAX_PACKET_SIZE};

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

            let mut test = false;
            if src.ip() == IpAddr::V4(Ipv4Addr::new(109, 60, 95, 182)) {
                test = true;
            }

            let response = decode_msg(&buf[..size], test);
            if response.is_some() {
                if test {
                    println!("Sending pong to {:?}", src);
                }
                let _ = socket
                    .send_to(
                        &create_disc_v4_packet(response.unwrap(), secret_key)[..],
                        src,
                    )
                    .await;
            }
        }
    }
}
