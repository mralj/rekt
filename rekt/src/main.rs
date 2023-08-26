use std::fs::File;
use std::sync::Arc;

use rekt::config::get_config;
use rekt::discover::server::run_discovery_server;
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

    let outbound_connections = Arc::new(OutboundConnections::new(config.nodes));
    OutboundConnections::start(outbound_connections).await;

    tokio::task::spawn(async move {
        let _ = run_discovery_server().await;
    });

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
