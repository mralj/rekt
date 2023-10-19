use std::str::FromStr;

use bytes::BytesMut;
use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use open_fastrlp::Header;
use tokio::sync::RwLock;

use crate::{
    cli::Cli,
    token::token::Token,
    utils::wei_gwei_converter::{gwei_to_wei, MIN_GAS_PRICE},
};

use super::{
    local_wallets_list::{LOCAL_WALLETS_LIST, PREPARE_WALLET_ADDRESS, SELL_WALLET_ADDRESS},
    wallet_with_nonce::{WalletWithNonce, WeiGasPrice},
};

pub static LOCAL_WALLETS: Lazy<RwLock<Vec<WalletWithNonce>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub static PREPARE_WALLET: Lazy<RwLock<WalletWithNonce>> = Lazy::new(|| {
    RwLock::new(
        WalletWithNonce::from_str(PREPARE_WALLET_ADDRESS).expect("Prepare wallet is invalid"),
    )
});

pub static SELL_WALLET: Lazy<RwLock<WalletWithNonce>> = Lazy::new(|| {
    RwLock::new(WalletWithNonce::from_str(SELL_WALLET_ADDRESS).expect("Sell wallet is invalid"))
});

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

    PREPARE_WALLET.write().await.update_nonce().await;
    if PREPARE_WALLET.read().await.nonce().is_none() {
        panic!("Prepare wallet has no nonce");
    }

    SELL_WALLET.write().await.update_nonce().await;
    if SELL_WALLET.read().await.nonce().is_none() {
        panic!("Sell wallet has no nonce");
    }
}

pub async fn update_nonces_for_local_wallets() {
    let mut local_wallets = LOCAL_WALLETS.write().await;

    let nonce_tasks =
        FuturesUnordered::from_iter(local_wallets.iter_mut().map(|wallet| wallet.update_nonce()));
    let _ = nonce_tasks.collect::<Vec<_>>().await;
}

pub async fn generate_and_rlp_encode_buy_txs_for_local_wallets(
    gas_price_in_wei: WeiGasPrice,
) -> BytesMut {
    let mut local_wallets = LOCAL_WALLETS.write().await;

    let generate_buy_txs_tasks = FuturesUnordered::from_iter(
        local_wallets
            .iter_mut()
            .map(|wallet| wallet.generate_and_sign_buy_tx(gas_price_in_wei)),
    );

    let buy_txs = generate_buy_txs_tasks
        .filter_map(|tx| async move { tx.ok() })
        .collect::<Vec<_>>()
        .await;

    let rlp_encoded_buy_txs = rlp_encode_list_of_bytes(&buy_txs);
    rlp_encoded_buy_txs
}

pub async fn generate_and_rlp_encode_prep_tx(token: &Token) -> BytesMut {
    let prep_wallet = &mut PREPARE_WALLET.write().await;
    prep_wallet.update_nonce().await;

    let tx = prep_wallet
        .generate_and_sign_prep_tx(token, gwei_to_wei(MIN_GAS_PRICE))
        .await
        .expect("Failed to generate and sign prep tx");

    rlp_encode_list_of_bytes(&vec![tx])
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
