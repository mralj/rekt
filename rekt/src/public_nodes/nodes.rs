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
            match p.get_block_number().await {
                Ok(b_no) => {
                    println!("Connected to public node: {rpc_url}, Highest known block {b_no}");
                    PUBLIC_NODES.lock().unwrap().push(p);
                }
                Err(e) => {
                    println!("Failed to connect to public node: {}", e);
                }
            }
        }
    }
}
