use std::time::Duration;

use ethers::{
    providers::{Http, JsonRpcClient, RetryClient, RetryClientBuilder},
    types::{Block, BlockNumber, Transaction, H256, U256},
    utils,
};
use futures::stream::FuturesUnordered;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use url::Url;

use crate::wallets::wallet_with_nonce::WalletWithNonce;

const DEFAULT_RETRY_COUNT: u8 = 2;
const DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC: u64 = 5;

pub const PUBLIC_NODE_URLS: [&str; 6] = [
    "https://bscrpc.com",
    "https://bsc-dataseed.binance.org/",
    "https://rpc.ankr.com/bsc",
    "https://bsc-dataseed1.defibit.io/",
    "https://bsc-dataseed1.ninicoin.io/",
    "https://bsc.nodereal.io",
];

static PUBLIC_NODES: Lazy<RwLock<Vec<RetryClient<Http>>>> = Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_connection_to_public_nodes() -> U256 {
    let mut highest_known_td = U256::zero();

    for rpc_url in PUBLIC_NODE_URLS.iter() {
        let mut public_nodes = PUBLIC_NODES.write().await;
        if let Ok(p) = get_retry_provider(rpc_url) {
            match JsonRpcClient::request::<_, Block<H256>>(
                &p,
                "eth_getBlockByNumber",
                [
                    utils::serialize::<BlockNumber>(&BlockNumber::Latest.into()),
                    utils::serialize::<bool>(&false),
                ],
            )
            .await
            {
                Ok(highest_block) => {
                    let td = highest_block.total_difficulty.unwrap_or_default();
                    if td > highest_known_td {
                        highest_known_td = td;
                    }
                    println!(
                        "Connected to public node: {rpc_url}, Highest known block {}, TD: {td}",
                        highest_block.number.unwrap_or_default()
                    );
                    public_nodes.push(p);
                }
                Err(e) => {
                    println!("Failed to connect to public node: {}", e);
                }
            }
        }
    }

    highest_known_td
}

pub fn get_retry_provider(url: &str) -> Result<RetryClient<Http>, url::ParseError> {
    let provider = Http::new(Url::parse(url)?);

    let client = RetryClientBuilder::default()
        .rate_limit_retries(DEFAULT_RETRY_COUNT.into())
        .timeout_retries(DEFAULT_RETRY_COUNT.into())
        .initial_backoff(Duration::from_secs(
            DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC,
        ))
        .build(
            provider,
            Box::<ethers::providers::HttpRateLimitRetryPolicy>::default(),
        );

    Ok(client)
}

pub async fn get_nonce(wallet: &WalletWithNonce) -> Option<U256> {
    let providers = PUBLIC_NODES.read().await;
    let mut nonce_tasks = FuturesUnordered::from_iter(providers.iter().map(|p| {
        tokio::time::timeout(
            std::time::Duration::from_secs(
                (DEFAULT_RETRY_COUNT as u64) * DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC,
            ),
            JsonRpcClient::request(
                p,
                "eth_getTransactionCount",
                [
                    utils::serialize(&wallet.address()),
                    utils::serialize::<BlockNumber>(&BlockNumber::Pending.into()),
                ],
            ),
        )
    }));

    if let Some(Ok(Ok(nonce))) = nonce_tasks.next().await {
        return Some(nonce);
    }

    None
}
