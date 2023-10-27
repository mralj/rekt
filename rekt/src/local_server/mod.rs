use std::str::FromStr;

use color_print::cprintln;
use derive_more::Display;
use ethers::types::Address;
use tokio::sync::broadcast;
use warp::{filters::path::end, reject::Reject, Filter};

use crate::{
    eth::eth_message::EthMessage,
    p2p::Peer,
    server::peers::PEERS,
    token::tokens_to_buy::{get_token_by_address, remove_all_tokens_to_buy},
    utils::wei_gwei_converter::MIN_GAS_PRICE,
    wallets::local_wallets::generate_and_rlp_encode_prep_tx,
};

pub fn run_local_server(send_txs_channel: broadcast::Sender<EthMessage>) {
    tokio::task::spawn(async move {
        //TODO: extract this into at least separate function (and maybe even file)
        let prep = warp::path!("prep" / String).and_then({
            move |token_address: String| async move {
                let token_address = match Address::from_str(&token_address) {
                    Ok(t_a) => t_a,
                    Err(e) => {
                        cprintln!("<red>Invalid token address</>: {}", e);
                        return Err(warp::reject::custom(LocalServerErr::InvalidTokenAddress));
                    }
                };

                let token = get_token_by_address(&token_address);
                if token.is_none() {
                    cprintln!("<red>Token not found</>");
                    return Err(warp::reject::custom(LocalServerErr::TokenNotFound));
                }
                let token = token.unwrap();
                let prep_tx = EthMessage::new_compressed_tx_message(
                    generate_and_rlp_encode_prep_tx(token, MIN_GAS_PRICE).await,
                );
                Peer::send_tx(prep_tx).await;
                cprintln!(
                    "<yellow>Prep sent successfully: {}</>",
                    token.buy_token_address
                );
                return Ok(format!(
                    "Prep sent successfully: {}",
                    token.buy_token_address
                ));
            }
        });

        let peer_count = warp::path!("peercount")
            .and(end())
            .map(|| format!("Peer count: {}\n", PEERS.len()));

        let refresh_tokens = warp::path("refreshtokens").and(end()).map(|| {
            tokio::task::spawn(async move {
                remove_all_tokens_to_buy();
                cprintln!("<yellow>Tokens refreshed</>");
            });
            format!("Tokens refreshed\n")
        });

        let routes = prep.or(peer_count).or(refresh_tokens);
        warp::serve(routes).run(([0, 0, 0, 0], 6060)).await;
    });
}

#[derive(Debug, Display)]
enum LocalServerErr {
    InvalidTokenAddress,
    TokenNotFound,
    TxChannelError,
}

impl Reject for LocalServerErr {}
