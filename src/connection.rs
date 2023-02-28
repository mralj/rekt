use secp256k1::SecretKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::constants::KB;
use crate::ecies::ecies::ECIES;
use crate::types::node_record::NodeRecord;

const CONN_CLOSED_FLAG: usize = 0;

pub fn connect_to_node(node: NodeRecord, secret_key: SecretKey) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ecies = ECIES::new(secret_key, node.pub_key);
        print!("Node {}", node.pub_key);
        let mut stream = match TcpStream::connect(node.get_socket_addr()).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                return;
            }
        };

        let mut wbuf = bytes::BytesMut::with_capacity(100 * KB);
        ecies.write_auth(&mut wbuf);

        match stream.write_all(&wbuf).await {
            Ok(_) => {
                println!("Sent auth");
            }
            Err(e) => {
                eprintln!("Failed to write to socket: {}", e);
            }
        }

        // TODO: look more into proper buffering. I'll do this when framing is implemented
        // and don't forget GO's implementation where they grow buffer so that buffer size is
        // basically max_msg_received_in_bytes
        // for tim being 100kb is randomly picked and should be ok
        let mut rbuf = bytes::BytesMut::with_capacity(100 * KB);
        loop {
            match stream.read(&mut rbuf).await {
                Ok(CONN_CLOSED_FLAG) => {
                    println!("Connection closed");
                    return;
                }
                Ok(_) => {
                    println!("Got msg: {}", String::from_utf8_lossy(&rbuf));
                }
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                }
            }
        }
    })
}
