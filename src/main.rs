use std::fs::File;
use std::sync::Arc;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use secp256k1::{Secp256k1, SecretKey};

use rekt::{constants::*, rlpx::connect_to_node};
use tokio::sync::Semaphore;
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let public_key = secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);

    let semaphore = Arc::new(Semaphore::new(1_000)); // Limit to 1000 concurrent connection attempts.
    let mut connect_to_nodes_tasks: FuturesUnordered<_> = RND_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .map(|n| connect_to_node(n, secret_key, public_key, semaphore.clone()))
        .collect();

    while let Some(task_result) = connect_to_nodes_tasks.next().await {
        if let Ok(Err(e)) = task_result {
            error!("{}", e)
        }
    }

    Ok(())
}
