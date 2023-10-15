use std::sync::Mutex;

use ethers::providers::{Http, Middleware, Provider};
use once_cell::sync::Lazy;

const PUBLIC_NODE_URLS: [&str; 6] = [
    "https://bscrpc.com",
    "https://bsc-dataseed.binance.org/",
    "https://rpc.ankr.com/bsc",
    "https://bsc-dataseed1.defibit.io/",
    "https://bsc-dataseed1.ninicoin.io/",
    "https://bsc.nodereal.io",
];

static PUBLIC_NODES: Lazy<Mutex<Vec<Provider<Http>>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub async fn init_connection_to_public_nodes() {
    for rpc_url in PUBLIC_NODE_URLS.iter() {
        if let Ok(p) = Provider::<Http>::try_from(*rpc_url) {
            if let Ok(node_info) = p.node_info().await {
                print!("Connected to node: {:?} ", node_info);
                PUBLIC_NODES.lock().unwrap().push(p);
            }
        }
    }
}
