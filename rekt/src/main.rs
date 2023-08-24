use std::fs::File;
use std::sync::Arc;

use rekt::config::get_config;
use rekt::server::inbound_connections::{self, InboundConnections};
use rekt::server::outbound_connections::OutboundConnections;

use secp256k1::SecretKey;
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

    let our_node_sk = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let our_node_pk =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &our_node_sk);

    let inbound_connections = InboundConnections::new(our_node_sk, our_node_pk);
    inbound_connections.start();

    let outbound_connections = Arc::new(OutboundConnections::new(
        config.nodes,
        our_node_sk,
        our_node_pk,
    ));
    OutboundConnections::start(outbound_connections).await;

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
