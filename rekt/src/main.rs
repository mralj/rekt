use std::fs::File;
use std::sync::Arc;

use rekt::cli::Cli;
use rekt::config::get_config;
use rekt::constants::BOOTSTRAP_NODES;
use rekt::local_node::LocalNode;
use rekt::local_server::run_local_server;
use rekt::mev;
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
    let _cpus = num_cpus::get(); // cache this

    let mut args = Cli::parse();

    mev::puissant::ping().await;
    mev::puissant::get_score().await;

    let mut config = get_config()?;
    let all_nodes = get_all_nodes(&mut config.nodes);

    rekt::eth::transactions::cache::init_cache();

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
    init_local_wallets(&mut args).await;

    import_tokens_to_buy();

    println!("{}", args);

    let (conn_tx, conn_rx) = tokio::sync::mpsc::unbounded_channel();
    let (tx_sender, _) = tokio::sync::broadcast::channel(2);
    let outbound_connections = OutboundConnections::new(
        our_node.private_key,
        our_node.public_key,
        all_nodes.clone(),
        conn_rx,
        conn_tx.clone(),
        args.clone(),
        tx_sender.clone(),
    );

    BLACKLIST_PEERS_BY_ID.insert(our_node.node_record.id);
    outbound_connections.run();

    let disc_server = if our_node.public_ip_retrieved {
        let (udp_tx, udp_rx) = tokio::sync::mpsc::unbounded_channel();
        let discover_server = Arc::new(
            rekt::discover::server::Server::new(
                our_node.clone(),
                all_nodes,
                conn_tx,
                udp_tx,
                args.clone(),
            )
            .await?,
        );
        rekt::discover::server::Server::start(discover_server.clone(), udp_rx);
        Some(discover_server)
    } else {
        println!("Failed to retrieve public ip, discovery server not started");
        None
    };

    let incoming_listener = Arc::new(InboundConnections::new(our_node, args, tx_sender.clone()));
    let listener = incoming_listener.clone();
    tokio::spawn(async move {
        if let Err(e) = listener.run().await {
            println!("Failed to run incoming connection listener: {}", e);
        }
    });

    run_local_server(disc_server, incoming_listener, tx_sender);

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
