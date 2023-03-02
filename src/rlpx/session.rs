use secp256k1::SecretKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::constants::KB;
use crate::rlpx::Connection;
use crate::types::node_record::NodeRecord;

const CONN_CLOSED_FLAG: usize = 0;

pub fn connect_to_node(node: NodeRecord, secret_key: SecretKey) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut rlpx_connection = Connection::new(secret_key, node.pub_key);
        let mut stream = match TcpStream::connect(node.get_socket_addr()).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                return;
            }
        };

        // TODO: look more into proper buffering. I'll do this when framing is implemented
        // for tim being 100kb is randomly picked and should be ok
        let mut buf = bytes::BytesMut::with_capacity(100 * KB);
        rlpx_connection.write_auth(&mut buf);

        match stream.write_all(&buf).await {
            Ok(_) => {
                println!("Sent auth");
            }
            Err(e) => {
                eprintln!("Failed to write to socket: {}", e);
            }
        }

        loop {
            match stream.read(&mut buf).await {
                Ok(CONN_CLOSED_FLAG) => {
                    println!("Connection closed");
                    return;
                }
                Ok(_) => {
                    if let Err(e) = rlpx_connection.read_ack(&mut buf) {
                        eprintln!("Failed to read ack: {}", e);
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                }
            }
        }
    })
}
