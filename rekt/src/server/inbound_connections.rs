use secp256k1::{PublicKey, SecretKey};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};

use crate::constants::DEFAULT_PORT;

#[derive(Clone)]
pub struct InboundConnections {
    nodes: Vec<String>,
    our_pub_key: PublicKey,
    our_private_key: secp256k1::SecretKey,
}

impl InboundConnections {
    pub fn new(our_node_sk: SecretKey, our_node_pk: PublicKey) -> Self {
        Self {
            nodes: Vec::new(),
            our_pub_key: our_node_pk,
            our_private_key: our_node_sk,
        }
    }

    pub fn start(&self) {
        let this = self.clone();
        tokio::task::spawn(async move { this.run_udp().await });

        let this = self.clone();
        tokio::task::spawn(async move { this.run_tcp().await });
    }

    async fn run_udp(&self) -> Result<(), io::Error> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_PORT)).await?;
        println!("UDP listening on 0.0.0.0:{}", DEFAULT_PORT);

        let mut buf = vec![0u8; 1280];
        loop {
            // Receive data into the buffer. This will wait until data is sent to the specified address.
            let req = socket.recv_from(&mut buf).await;
            match req {
                Ok((size, src)) => {
                    println!("Received from {}, data: {:?}", src, &buf[..size]);
                    // Echo the data back to the sender
                    socket.send_to(&buf[..size], &src).await?;
                }
                Err(e) => {
                    println!("failed to receive from socket; err = {:?}", e);
                }
            }
        }
    }

    async fn run_tcp(&self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", DEFAULT_PORT)).await?;
        println!("TCP Server listening on 0.0.0.0:{}", DEFAULT_PORT);

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
}
