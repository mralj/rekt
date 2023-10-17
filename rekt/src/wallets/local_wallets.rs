use std::str::FromStr;

use bytes::BytesMut;
use ethers::types::U256;
use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use open_fastrlp::Header;
use tokio::sync::RwLock;

use crate::cli::Cli;

use super::{local_wallets_list::LOCAL_WALLETS_LIST, wallet_with_nonce::WalletWithNonce};

pub static LOCAL_WALLETS: Lazy<RwLock<Vec<WalletWithNonce>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_local_wallets(args: &Cli) {
    let mut local_wallets = LOCAL_WALLETS_LIST
        .iter()
        //note server_index is counted from 1 not 0
        .skip((args.server_index - 1) * args.pings_per_server)
        .take(args.pings_per_server)
        .filter_map(|pk| WalletWithNonce::from_str(pk).ok())
        .collect::<Vec<WalletWithNonce>>();

    if local_wallets.len() != args.pings_per_server {
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

pub async fn generate_and_rlp_encode_buy_txs_for_local_wallets(gas_price_in_gwei: u64) -> BytesMut {
    let mut local_wallets = LOCAL_WALLETS.write().await;

    let generate_buy_txs_tasks = FuturesUnordered::from_iter(
        local_wallets
            .iter_mut()
            .map(|wallet| wallet.generate_and_sign_buy_tx(gwei_to_wei(gas_price_in_gwei))),
    );

    let buy_txs = generate_buy_txs_tasks
        .filter_map(|tx| async move { tx.ok() })
        .collect::<Vec<_>>()
        .await;

    let rlp_encoded_buy_txs = rlp_encode_list_of_bytes(&buy_txs);
    rlp_encoded_buy_txs
}

fn rlp_encode_list_of_bytes(txs_rlp_encoded: &[ethers::types::Bytes]) -> bytes::BytesMut {
    let mut out = BytesMut::with_capacity(txs_rlp_encoded.len() * 2);
    Header {
        list: true,
        payload_length: txs_rlp_encoded.iter().map(|tx| tx.len()).sum::<usize>(),
    }
    .encode(&mut out);
    txs_rlp_encoded
        .into_iter()
        .for_each(|tx| out.extend_from_slice(tx));

    out
}

fn gwei_to_wei(gwei: u64) -> U256 {
    U256::from(gwei) * U256::exp10(9)
}
