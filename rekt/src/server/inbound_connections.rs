use secp256k1::{PublicKey, SecretKey};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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

    pub async fn run(&self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", DEFAULT_PORT)).await?;
        println!("Server listening on 127.0.0.1:{}", DEFAULT_PORT);

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
