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

    pub async fn run(&self) -> Result<(), io::Error> {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", DEFAULT_PORT)).await?;
        println!("Server listening on 127.0.0.1:{}", DEFAULT_PORT);

        let mut buf = vec![0u8; 1280];
        loop {
            // Receive data into the buffer. This will wait until data is sent to the specified address.
            let (size, src) = socket.recv_from(&mut buf).await?;
            println!("Received from {}, data: {:?}", src, &buf[..size]);
            // Echo the data back to the sender
            socket.send_to(&buf[..size], &src).await?;
        }
    }
}
