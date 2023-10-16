use std::str::FromStr;

use ethers::{
    middleware::MiddlewareBuilder,
    prelude::k256::ecdsa::SigningKey,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, Wallet},
    types::{BlockNumber, U256},
};
use futures::stream::FuturesUnordered;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use crate::wallets::wallet_with_nonce::WalletWithNonce;

const DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC: u64 = 5;

const PUBLIC_NODE_URLS: [&str; 6] = [
    "https://bscrpc.com",
    "https://bsc-dataseed.binance.org/",
    "https://rpc.ankr.com/bsc",
    "https://bsc-dataseed1.defibit.io/",
    "https://bsc-dataseed1.ninicoin.io/",
    "https://bsc.nodereal.io",
];

static PUBLIC_NODES: Lazy<RwLock<Vec<Provider<Http>>>> = Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_connection_to_public_nodes() {
    for rpc_url in PUBLIC_NODE_URLS.iter() {
        let mut public_nodes = PUBLIC_NODES.write().await;
        if let Ok(p) = Provider::<Http>::try_from(*rpc_url) {
            match p.get_block_number().await {
                Ok(b_no) => {
                    println!("Connected to public node: {rpc_url}, Highest known block {b_no}");
                    public_nodes.push(p);
                }
                Err(e) => {
                    println!("Failed to connect to public node: {}", e);
                }
            }
        }
    }
}

pub async fn get_nonces() {
    let private_keys = vec![
        "c9aebfba092f657150d66df2ec450e56b3d36cbb6c2f54c517208f003019d075",
        "72f6f94063935787b1a3a10e97fe5300c1d1da237dd0a1b0b83f5676a87f1a41",
        "88565bd29f41084bb57333ec4f458df9647f7a2216643a9670cd6ce5dfdde52b",
        "a4166e95e71f53ad469144eb034aa1beee517cd14513dbb49743fe9ee29839b2",
        "0af54ac661e593d6b3d34d3e0366e0c221651cce8b518cf8424cf9260ce6ace3",
        "bd1bbd7a99228e2cc40e589474d3b7d7393751b0edd669d9af342992394621be",
        "e22b68b87e5b52479dc4c9818ce3840aec79e917ed3fc3ab33639c437d6b4b90",
        "387f73baa0e605b91cba78386c2b303db36b59d5447f1286e1b7689f7f929036",
        "fc3dce9c1b1958f3d6b6944f988c2d2d216468cafa8ae48a4ae17ddc96d06806",
    ];

    let wallets: Vec<WalletWithNonce> = private_keys
        .iter()
        .filter_map(|k| WalletWithNonce::from_str(k).ok())
        .collect();

    if wallets.len() != private_keys.len() {
        panic!("Failed to parse all private keys");
    }

    let mut nonce_tasks = FuturesUnordered::from_iter(wallets.iter().map(|w| get_nonce(w)));
    while let Some(nonce) = nonce_tasks.next().await {
        println!("Nonce: {:?}", nonce);
        let _n = nonce;
    }
}

pub async fn get_nonce(wallet: &WalletWithNonce) -> Option<U256> {
    let providers = PUBLIC_NODES.read().await;
    let mut nonce_tasks = FuturesUnordered::from_iter(providers.iter().map(|p| {
        tokio::time::timeout(
            std::time::Duration::from_secs(DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC),
            p.get_transaction_count(wallet.address(), Some(BlockNumber::Pending.into())),
        )
    }));

    if let Some(Ok(Ok(nonce))) = nonce_tasks.next().await {
        return Some(nonce);
    }

    return None;
}
