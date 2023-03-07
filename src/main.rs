use futures::future::join_all;
use secp256k1::SecretKey;
use tokio::task::JoinHandle;

use rekt::{constants::*, rlpx::connect_to_node};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(collector).expect("Could not init tracing");

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let connect_to_nodes_tasks = BOOTSTRAP_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .into_iter()
        .map(|n| connect_to_node(n, secret_key))
        .collect::<Vec<JoinHandle<()>>>();

    join_all(connect_to_nodes_tasks).await;

    Ok(())
}
