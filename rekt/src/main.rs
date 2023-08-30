use std::fs::File;
use std::sync::Arc;

use rekt::config::get_config;
use rekt::discover::server::DiscoveryServer;
use rekt::local_node::LocalNode;
use rekt::server::outbound_connections::OutboundConnections;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config()?;

    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let our_node = LocalNode::new(public_ip::addr().await);

    println!("{:?}", our_node.node_record.str);

    let outbound_connections = Arc::new(OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        config.nodes,
    ));
    OutboundConnections::start(outbound_connections).await;

    if our_node.public_ip_retrieved {
        tokio::task::spawn(async move {
            if let Ok(disc_server) = DiscoveryServer::new(our_node.clone()).await {
                disc_server.start().await
            } else {
                println!("Failed to start discovery server");
            }
        });
    } else {
        println!("Failed to retrieve public ip, discovery server not started");
    }

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
