use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rekt::config::get_config;
use rekt::server::outbound_connections::OutboundConnections;
use rekt::types::hash::H512;
use secp256k1::{Secp256k1, SecretKey};

use rekt::rlpx::connect_to_node;
use tokio::select;
use tokio::sync::Semaphore;
use tokio::time::interval;
use tracing::{error, info, Level};
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

    let peers = Arc::new(Mutex::new(HashMap::<H512, String>::with_capacity(
        2 * config.nodes.len(),
    )));

    let peers_c = peers.clone();
    tokio::spawn(async move {
        let mut count_interval = interval(Duration::from_secs(10));
        let mut info_interval = interval(Duration::from_secs(10 * 60));

        loop {
            select! {
                _ = count_interval.tick() => {
                    let p = peers_c.lock().unwrap();
                    println!("{}", p.len());
                },
                _ = info_interval.tick() => {
                    info!("==================== ==========================  ==========");
                    for (_, v)  in peers_c.lock().unwrap().iter() {
                        info!("{}", v);
                    }
                }
            }
        }
    });

    let outbound_connections = OutboundConnections::new(config.nodes, peers);
    outbound_connections.start().await;

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
