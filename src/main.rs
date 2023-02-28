use color_eyre::eyre::Result;
use rekt::connection::connect_to_node;
use rekt::constants::*;
use secp256k1::SecretKey;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?; // better errors

    let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
    let connect_to_nodes_tasks = BOOTSTRAP_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .into_iter()
        .map(|n| connect_to_node(n, secret_key))
        .collect::<Vec<JoinHandle<()>>>();

    for t in connect_to_nodes_tasks {
        t.await?;
    }

    Ok(())
}
