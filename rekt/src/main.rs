use std::fs::File;
use std::sync::Arc;

use rekt::config::get_config;
use rekt::discover::server::run_discovery_server;
use rekt::server::outbound_connections::OutboundConnections;

use rekt::types::node_record::NodeRecord;
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

    let our_private_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let our_pub_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &our_private_key);

    println!("{:?}", NodeRecord::get_local_node(our_pub_key).str);

    if let Some(ip) = public_ip::addr().await {
        println!("public ip address: {:?}", ip);
    } else {
        println!("couldn't get an IP address");
    }

    let outbound_connections = Arc::new(OutboundConnections::new(
        our_private_key,
        our_pub_key,
        config.nodes,
    ));
    OutboundConnections::start(outbound_connections).await;

    tokio::task::spawn(async move {
        let _ = run_discovery_server(&our_private_key).await;
    });

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
