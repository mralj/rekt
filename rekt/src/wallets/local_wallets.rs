use std::str::FromStr;

use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::cli::Cli;

use super::{local_wallets_list::LOCAL_WALLETS_LIST, wallet_with_nonce::WalletWithNonce};

pub static LOCAL_WALLETS: Lazy<RwLock<Vec<WalletWithNonce>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_local_wallets(args: &Cli) {
    let mut local_wallets = LOCAL_WALLETS_LIST
        .iter()
        //note server_index is counted from 1 not 0
        .skip(((args.server_index - 1) * args.pings_per_server) as usize)
        .take(args.pings_per_server as usize)
        .filter_map(|pk| WalletWithNonce::from_str(pk).ok())
        .collect::<Vec<WalletWithNonce>>();

    if local_wallets.len() != args.pings_per_server as usize {
        panic!("Some local wallets are invalid");
    }

    let nonce_tasks =
        FuturesUnordered::from_iter(local_wallets.iter_mut().map(|wallet| wallet.update_nonce()));
    let _ = nonce_tasks.collect::<Vec<_>>().await;

    if local_wallets.iter().any(|wallet| wallet.nonce().is_none()) {
        println!("Some local wallets have no nonce");
    }

    *LOCAL_WALLETS.write().await = local_wallets;
}
