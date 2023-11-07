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
use mimalloc::MiMalloc;
use rekt::token::tokens_to_buy::import_tokens_to_buy;
use rekt::wallets::local_wallets::init_local_wallets;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("{}", args);
    let mut config = get_config()?;

    init_cache();
    println!("TX cache initialized");

    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let our_node = LocalNode::new(public_ip::addr().await);
    println!("{:?}", our_node.node_record.str);

    init_connection_to_public_nodes().await;
    init_local_wallets(&args).await;

    import_tokens_to_buy();

    let all_nodes = get_all_nodes(&mut config.nodes);
    let (conn_tx, conn_rx) = kanal::unbounded_async();
    let outbound_connections = Arc::new(OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        all_nodes.clone(),
        conn_rx,
        conn_tx.clone(),
    ));

    OutboundConnections::start(outbound_connections).await;

    let disc_server = if our_node.public_ip_retrieved {
        let discover_server =
            Arc::new(rekt::discover::server::Server::new(our_node, all_nodes, conn_tx).await?);
        rekt::discover::server::Server::start(discover_server.clone());
        Some(discover_server)
    } else {
        println!("Failed to retrieve public ip, discovery server not started");
        None
    };

    run_local_server(disc_server);

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
