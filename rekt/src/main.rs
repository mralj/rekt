use std::fs::File;
use std::sync::Arc;

use rekt::cli::Cli;
use rekt::config::get_config;
use rekt::constants::BOOTSTRAP_NODES;
use rekt::eth::transactions::cache::init_cache;
use rekt::local_node::LocalNode;
use rekt::local_server::run_local_server;
use rekt::public_nodes::nodes::init_connection_to_public_nodes;
use rekt::server::outbound_connections::OutboundConnections;

use clap::Parser;
use rekt::token::tokens_to_buy::import_tokens_to_buy;
use rekt::wallets::local_wallets::init_local_wallets;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_cache();

    let args = Cli::parse();
    let mut config = get_config()?;

    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let (tx_sender, _) = tokio::sync::broadcast::channel(1);
    run_local_server(tx_sender.clone());
    let our_node = LocalNode::new(public_ip::addr().await);

    println!("{:?}", our_node.node_record.str);

    init_connection_to_public_nodes().await;
    init_local_wallets(&args).await;

    import_tokens_to_buy();

    let outbound_connections = Arc::new(OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        get_all_nodes(&mut config.nodes),
        tx_sender,
    ));

    OutboundConnections::start(outbound_connections).await;

    // if our_node.public_ip_retrieved {
    //     let discover_server = Arc::new(rekt::discover::server::Server::new(our_node).await?);
    //     rekt::discover::server::Server::start(discover_server);
    // } else {
    //     println!("Failed to retrieve public ip, discovery server not started");
    // }

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}

fn get_all_nodes(static_nodes: &mut Vec<String>) -> Vec<String> {
    let mut nodes = BOOTSTRAP_NODES
        .iter()
        .copied()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    nodes.append(static_nodes);
    nodes
}
