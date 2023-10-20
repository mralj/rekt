use std::str::FromStr;

use color_print::cprintln;
use derive_more::Display;
use ethers::types::Address;
use tokio::sync::broadcast;
use warp::{filters::path::end, reject::Reject, Filter};

use crate::{
    eth::eth_message::EthMessage, server::peers::PEERS, token::tokens_to_buy::get_token_by_address,
    wallets::local_wallets::generate_and_rlp_encode_prep_tx,
};

pub fn run_local_server(send_txs_channel: broadcast::Sender<EthMessage>) {
    tokio::task::spawn(async move {
        //TODO: extract this into at least separate function (and maybe even file)
        let prep = warp::path!("prep" / String).and_then({
            let send_txs_channel = send_txs_channel.clone();
            move |token_address: String| {
                let send_txs_channel = send_txs_channel.clone();
                async move {
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
                    match send_txs_channel.send(EthMessage::new_tx_message(
                        generate_and_rlp_encode_prep_tx(token).await,
                    )) {
                        Ok(_) => {
                            cprintln!(
                                "<yellow>Prep sent successfully: {}</>",
                                token.buy_token_address
                            );
                            return Ok(format!(
                                "Prep sent successfully: {}",
                                token.buy_token_address
                            ));
                        }
                        Err(e) => {
                            cprintln!("<red> Channel error: {e}</>");
                            return Err(warp::reject::custom(LocalServerErr::TxChannelError));
                        }
                    }
                }
            }
        });

        let peer_count = warp::path!("peercount")
            .and(end())
            .map(|| format!("Peer count: {}\n", PEERS.len()));

        let routes = prep.or(peer_count);
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
