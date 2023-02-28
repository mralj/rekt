use color_eyre::eyre::Result;
use rekt::connection::connect_to_node;
use rekt::constants::*;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?; // better errors

    let connect_to_nodes_tasks = BOOTSTRAP_NODES
        .iter()
        .map(|n| n.parse().unwrap())
        .into_iter()
        .map(connect_to_node)
        .collect::<Vec<JoinHandle<()>>>();

    for t in connect_to_nodes_tasks {
        t.await?;
    }

    Ok(())
}
