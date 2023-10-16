use std::str::FromStr;

use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use super::{local_wallets_list::LOCAL_WALLETS_LIST, wallet_with_nonce::WalletWithNonce};

pub static LOCAL_WALLETS: Lazy<RwLock<Vec<WalletWithNonce>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_local_wallets() {
    let mut local_wallets = LOCAL_WALLETS_LIST
        .iter()
        .filter_map(|pk| WalletWithNonce::from_str(pk).ok())
        .collect::<Vec<WalletWithNonce>>();

    if local_wallets.len() != LOCAL_WALLETS_LIST.len() {
        panic!("Some local wallets are invalid");
    }

    let nonce_tasks =
        FuturesUnordered::from_iter(local_wallets.iter_mut().map(|wallet| wallet.update_nonce()));
    let _ = nonce_tasks.collect::<Vec<_>>().await;

    let cnt = local_wallets
        .iter()
        .filter(|wallet| wallet.nonce().is_none())
        .count();

    println!("{} local wallets have no nonce", cnt);

    *LOCAL_WALLETS.write().await = local_wallets;
}
