use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpSocket,
};

use crate::constants::DEFAULT_PORT;

pub async fn run_incoming_connection_listener() -> Result<(), io::Error> {
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
