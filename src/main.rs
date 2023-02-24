use color_eyre::eyre::Result;
use rekt::{constants::*, types::node_record::NodeRecord};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?; // better errors

    let nodes = RND_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .collect::<Vec<NodeRecord>>();

    let mut tasks = Vec::new();
    for node in nodes {
        let t = tokio::spawn(async move {
            let mut stream = match TcpStream::connect(node.get_socket_addr()).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to connect: {}", e);
                    return;
                }
            };
            // 5kb is random
            let mut buf = [0; 100 * 1024];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        println!("Connection closed");
                        return;
                    }
                    Ok(_) => {
                        println!("The server says {}", String::from_utf8_lossy(&buf));
                    }
                    Err(e) => {
                        eprintln!("Failed to read from socket: {}", e);
                    }
                }
            }
        });
        tasks.push(t);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}
