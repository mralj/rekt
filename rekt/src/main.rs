use std::fs::File;
use std::sync::Arc;
use std::time::Duration;

use rekt::config::get_config;
use rekt::constants::BOOTSTRAP_NODES;
use rekt::discover::server::{run_tcp, DiscoveryServer};
use rekt::local_node::LocalNode;
use rekt::server::outbound_connections::OutboundConnections;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = get_config()?;

    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let our_node = LocalNode::new(public_ip::addr().await);

    println!("{:?}", our_node.node_record.str);

    let mut nodes: Vec<String> = BOOTSTRAP_NODES.iter().cloned().map(String::from).collect();
    nodes.append(&mut config.nodes);

    let outbound_connections = Arc::new(OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        &config.nodes,
    ));
    OutboundConnections::start(outbound_connections).await;

    if our_node.public_ip_retrieved {
        tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            match DiscoveryServer::new(our_node.clone(), nodes).await {
                Ok(disc_server) => disc_server.start().await,
                Err(e) => println!("Failed to start discovery server: {:?}", e),
            }
        });
        tokio::task::spawn(async move {
            let _ = run_tcp().await;
        });
    } else {
        println!("Failed to retrieve public ip, discovery server not started");
    }

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
