use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rekt::config::get_config;
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

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let public_key = secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);

    let semaphore = Arc::new(Semaphore::new(1_000)); // Limit to 1000 concurrent connection attempts.
    let mut connect_to_nodes_tasks: FuturesUnordered<_> = config
        .nodes
        .iter()
        .map(|n| n.parse().unwrap())
        .map(|n| connect_to_node(n, secret_key, public_key, semaphore.clone(), peers.clone()))
        .collect();

    while let Some(task_result) = connect_to_nodes_tasks.next().await {
        if let Ok(Err(e)) = task_result {
            error!("{}", e)
        }
    }

    Ok(())
}
