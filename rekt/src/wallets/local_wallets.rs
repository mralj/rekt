use std::str::FromStr;

use bytes::{Bytes, BytesMut};
use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use open_fastrlp::Header;
use tokio::sync::RwLock;

use crate::{
    cli::Cli,
    eth::types::protocol::{EthProtocol, ETH_PROTOCOL_OFFSET},
    token::token::Token,
    utils::wei_gwei_converter::{
        gwei_to_wei, gwei_to_wei_with_decimals, DEFAULT_GWEI_DECIMAL_PRECISION, MIN_GAS_PRICE,
    },
};

use super::{
    local_wallets_list::{
        LOCAL_WALLETS_LIST, PREPARE_WALLET_ADDRESS, PRIORITY_WALLET_ADDRESS, SELL_WALLET_ADDRESS,
        UN_IMPORTANT_WALLETS_START_AT_INDEX,
    },
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

pub static PRIORITY_WALLET: Lazy<RwLock<WalletWithNonce>> = Lazy::new(|| {
    RwLock::new(
        WalletWithNonce::from_str(PRIORITY_WALLET_ADDRESS).expect("Priority wallet is invalid"),
    )
});

pub async fn init_local_wallets(args: &mut Cli) {
    let first_wallet_index = if args.is_un_important_server {
        //note server_index is counted from 1 not 0
        UN_IMPORTANT_WALLETS_START_AT_INDEX
            + (args.server_index - 1) * args.pings_per_unimportant_server
    } else {
        (args.server_index - 1) * args.pings_per_server
    };

    let mut local_wallets = LOCAL_WALLETS_LIST
        .iter()
        .skip(first_wallet_index)
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

    args.set_first_last_wallets(
        local_wallets.first().unwrap().address(),
        local_wallets.last().unwrap().address(),
    );

    *LOCAL_WALLETS.write().await = local_wallets;

    PREPARE_WALLET.write().await.update_nonce().await;
    if PREPARE_WALLET.read().await.nonce().is_none() {
        panic!("Prepare wallet has no nonce");
    }

    SELL_WALLET.write().await.update_nonce().await;
    if SELL_WALLET.read().await.nonce().is_none() {
        panic!("Sell wallet has no nonce");
    }

    PRIORITY_WALLET.write().await.update_nonce().await;
    if PRIORITY_WALLET.read().await.nonce().is_none() {
        panic!("Priority wallet has no nonce");
    }
}

pub async fn update_nonces_for_local_wallets() {
    let mut local_wallets = LOCAL_WALLETS.write().await;

    let nonce_tasks =
        FuturesUnordered::from_iter(local_wallets.iter_mut().map(|wallet| wallet.update_nonce()));
    let _ = nonce_tasks.collect::<Vec<_>>().await;

    let prep_wallet = &mut PREPARE_WALLET.write().await;
    prep_wallet.update_nonce().await;

    let sell_wallet = &mut SELL_WALLET.write().await;
    sell_wallet.update_nonce().await;

    let priority_wallet = &mut PRIORITY_WALLET.write().await;
    priority_wallet.update_nonce().await;
}

pub async fn generate_and_rlp_encode_buy_txs_for_local_wallets(
    token: &Token,
    gas_price_in_wei: WeiGasPrice,
) -> Bytes {
    let mut local_wallets = LOCAL_WALLETS.write().await;

    let generate_buy_txs_tasks = FuturesUnordered::from_iter(
        local_wallets
            .iter_mut()
            .map(|wallet| wallet.generate_and_sign_buy_tx(gas_price_in_wei)),
    );

    let mut buy_txs = generate_buy_txs_tasks
        .filter_map(|tx| async move { tx.ok() })
        .collect::<Vec<_>>()
        .await;

    if token.prep_in_flight {
        let prep_wallet = &mut PREPARE_WALLET.write().await;
        let prep_tx = prep_wallet
            .generate_and_sign_prep_tx(token, gas_price_in_wei + 1)
            .await
            .expect("Failed to generate and sign prep tx");

        buy_txs.push(prep_tx);
    }

    if let Some(priority_tx) = &token.priority_tx {
        let priority_wallet = &mut PRIORITY_WALLET.write().await;
        let priority_tx = priority_wallet
            .generate_and_sign_buy_tx(Token::get_gas_price_for_high_priority_tx(
                priority_tx.min_gas_price,
                priority_tx.max_gas_price,
            ))
            .await
            .expect("Failed to generate and sign priority tx");

        buy_txs.push(priority_tx);
    }
    snappy_compress_rlp_bytes(rlp_encode_list_of_bytes(&buy_txs))
}

pub async fn generate_rlp_snappy_prep_tx(token: &Token, gwei_gas_price: u64) -> Bytes {
    let tx = generate_rlp_prep_tx(token, gwei_gas_price).await;
    snappy_compress_rlp_bytes(rlp_encode_list_of_bytes(&vec![tx]))
}

pub async fn generate_rlp_prep_tx(token: &Token, gwei_gas_price: u64) -> ethers::types::Bytes {
    let prep_wallet = &mut PREPARE_WALLET.write().await;
    prep_wallet.update_nonce().await;

    let tx = prep_wallet
        .generate_and_sign_prep_tx(token, gwei_to_wei(gwei_gas_price))
        .await
        .expect("Failed to generate and sign prep tx");

    tx
}

pub async fn generate_and_rlp_encode_sell_tx(should_increment_nocne_locally: bool) -> Bytes {
    let sell_wallet = &mut SELL_WALLET.write().await;
    if should_increment_nocne_locally {
        sell_wallet.update_nonce_locally();
    }
    let tx = sell_wallet
        .generate_and_sign_sell_tx(gwei_to_wei(MIN_GAS_PRICE))
        .await
        .expect("Failed to generate and sign sell tx");

    rlp_encode_list_of_bytes(&vec![tx])
}

pub async fn generate_mev_bid(gas_price_in_gwei: u64) -> Bytes {
    let high_priority_wallet = &mut PRIORITY_WALLET.write().await;
    let tx = high_priority_wallet
        .generate_mev_tx(gwei_to_wei(gas_price_in_gwei))
        .await
        .expect("Failed to generate and sign mev tx");
    rlp_encode_list_of_bytes(&vec![tx])
}

fn rlp_encode_list_of_bytes(txs_rlp_encoded: &[ethers::types::Bytes]) -> bytes::Bytes {
    let mut out = BytesMut::with_capacity(txs_rlp_encoded.len() * 2);
    Header {
        list: true,
        payload_length: txs_rlp_encoded.iter().map(|tx| tx.len()).sum::<usize>(),
    }
    .encode(&mut out);
    txs_rlp_encoded
        .into_iter()
        .for_each(|tx| out.extend_from_slice(tx));

    out.freeze()
}

fn snappy_compress_rlp_bytes(rlp_tx: Bytes) -> Bytes {
    let mut snappy_encoder = snap::raw::Encoder::new();
    let mut compressed = BytesMut::zeroed(1 + snap::raw::max_compress_len(rlp_tx.len()));
    let compressed_size = snappy_encoder
        .compress(&rlp_tx, &mut compressed[1..])
        .expect("Failed to snappy compress tx");

    compressed[0] = EthProtocol::TransactionsMsg as u8 + ETH_PROTOCOL_OFFSET;
    compressed.truncate(compressed_size + 1);

    compressed.freeze()
}
