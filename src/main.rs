use futures::future::join_all;
use secp256k1::{Secp256k1, SecretKey};
use tokio::task::JoinHandle;

use rekt::{constants::*, rlpx::connect_to_node, rlpx::RLPXSessionError};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(collector).expect("Could not init tracing");

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let public_key = secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);
    let connect_to_nodes_tasks = RND_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .map(|n| connect_to_node(n, secret_key, public_key))
        .collect::<Vec<JoinHandle<Result<(), RLPXSessionError>>>>();

    join_all(connect_to_nodes_tasks)
        .await
        .iter()
        .filter(|e| e.is_err())
        .for_each(|e| eprintln!("Error: {:?}", e));

    Ok(())
}
