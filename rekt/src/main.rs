use std::fs::File;
use std::sync::Arc;

use rekt::cli::Cli;
use rekt::config::get_config;
use rekt::constants::BOOTSTRAP_NODES;
use rekt::local_node::LocalNode;
use rekt::local_server::run_local_server;
use rekt::public_nodes::nodes::init_connection_to_public_nodes;
use rekt::server::inbound_connections::InboundConnections;
use rekt::server::outbound_connections::OutboundConnections;

use clap::Parser;
use mimalloc::MiMalloc;
use rekt::server::peers::BLACKLIST_PEERS_BY_ID;
use rekt::token::tokens_to_buy::import_tokens_to_buy;
use rekt::types::node_record::NodeRecord;
use rekt::wallets::local_wallets::init_local_wallets;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Cli::parse();
    let td = init_connection_to_public_nodes().await;
    args.set_td(td);
    println!("{}", args);

    let mut config = get_config()?;
    let all_nodes = get_all_nodes(&mut config.nodes);

    rekt::eth::transactions::cache::init_cache();
    rekt::p2p::p2p_wire_cache::init_cache();

    let file = File::create("log.txt")?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not init tracing");

    let our_node = LocalNode::new(public_ip::addr().await);
    println!("{:?}", our_node.node_record.str);

    init_local_wallets(&mut args).await;

    import_tokens_to_buy();

    let (conn_tx, conn_rx) = kanal::unbounded_async();
    let outbound_connections = Arc::new(OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        all_nodes.clone(),
        conn_rx,
        conn_tx.clone(),
        args.clone(),
    ));

    BLACKLIST_PEERS_BY_ID.insert(our_node.node_record.id);
    OutboundConnections::start(outbound_connections).await;

    let disc_server = if our_node.public_ip_retrieved {
        let discover_server = Arc::new(
            rekt::discover::server::Server::new(our_node.clone(), all_nodes, conn_tx).await?,
        );
        rekt::discover::server::Server::start(discover_server.clone());
        Some(discover_server)
    } else {
        println!("Failed to retrieve public ip, discovery server not started");
        None
    };

    let incoming_listener = Arc::new(InboundConnections::new(our_node, args));
    let listener = incoming_listener.clone();
    tokio::spawn(async move {
        if let Err(e) = listener.run().await {
            println!("Failed to run incoming connection listener: {}", e);
        }
    });

    run_local_server(disc_server, incoming_listener);

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
    nodes.sort_unstable();
    nodes.dedup();
    let nodes = nodes
        .iter()
        .filter(|n| n.parse::<NodeRecord>().is_ok())
        .cloned()
        .collect::<Vec<String>>();
    println!("All nodes: {:?}", nodes.len());
    nodes
}
