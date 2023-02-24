use rekt::{constants::BOOTSTRAP_NODES, types::node_record::NodeRecord};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nodes = BOOTSTRAP_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .collect::<Vec<NodeRecord>>();

    for node in nodes {
        tokio::spawn(async move {
            let mut stream = match TcpStream::connect(node.get_socket_addr()).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to connect: {}", e);
                    return;
                }
            };
            // 5kb is random
            let mut buf = [0; 5 * 1024 * 1024];
            stream.read(&mut buf).await.unwrap();
            println!("Received: {}", String::from_utf8_lossy(&buf));
        });
    }

    Ok(())
}
