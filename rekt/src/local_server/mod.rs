use std::{str::FromStr, sync::Arc};

use color_print::cprintln;
use derive_more::Display;
use ethers::types::Address;
use warp::{filters::path::end, reject::Reject, Filter};

use crate::{
    discover::server::Server,
    eth::eth_message::EthMessage,
    p2p::{peer::PeerType, Peer},
    server::{inbound_connections::InboundConnections, peers::PEERS},
    token::tokens_to_buy::{get_token_by_address, remove_all_tokens_to_buy},
    utils::wei_gwei_converter::MIN_GAS_PRICE,
    wallets::local_wallets::generate_and_rlp_encode_prep_tx,
};

pub fn run_local_server(
    disc_server: Option<Arc<Server>>,
    incoming_listener: Arc<InboundConnections>,
) {
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
                let cnt = Peer::send_tx(prep_tx).await;
                cprintln!(
                    "<yellow>[{cnt}]Prep sent successfully: {}</>",
                    token.buy_token_address
                );
                return Ok(format!(
                    "Prep sent successfully: {}",
                    token.buy_token_address
                ));
            }
        });

        let peer_count = warp::path!("peercount").and(end()).map(|| {
            let mut inbound = 0;
            let mut outbound = 0;
            for p in PEERS.iter() {
                if p.value().peer_type == PeerType::Inbound {
                    inbound += 1;
                } else {
                    outbound += 1;
                }
            }
            format!(
                "Total: {}, Inbound: {}, Outbound: {}",
                inbound + outbound,
                inbound,
                outbound
            )
        });

        let refresh_tokens = warp::path("refreshtokens").and(end()).map(|| {
            tokio::task::spawn(async move {
                remove_all_tokens_to_buy();
                cprintln!("<yellow>Tokens refreshed</>");
            });
            format!("Tokens refreshed\n")
        });

        let disc = warp::path!("disc").and(end()).map(move || {
            if let Some(disc) = &disc_server {
                if disc.is_paused() {
                    disc.start_disc_server();
                    cprintln!("<yellow>Discovery server was off now it's ON</>");
                } else {
                    disc.stop_disc_server();
                    cprintln!("<yellow>Discovery server was ON, nof it's OFF</>");
                }
            } else {
                cprintln!("<yellow>Discovery server not found</>");
                return format!("Discovery server not found\n");
            }

            if incoming_listener.is_paused() {
                incoming_listener.start_listener();
                cprintln!("<yellow>Listener server was off now it's ON</>");
            } else {
                incoming_listener.stop_listener();
                cprintln!("<yellow>Listener server was on now it's OFF</>");
            }

            format!("Discovery&Listener servers toggled\n")
        });

        let routes = prep.or(peer_count).or(refresh_tokens).or(disc);
        warp::serve(routes).run(([0, 0, 0, 0], 6060)).await;
    });
}

#[derive(Debug, Display)]
enum LocalServerErr {
    InvalidTokenAddress,
    TokenNotFound,
}

impl Reject for LocalServerErr {}
