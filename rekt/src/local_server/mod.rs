use std::{str::FromStr, sync::Arc};

use color_print::cprintln;
use derive_more::Display;
use ethers::types::Address;
use warp::{filters::path::end, reject::Reject, Filter};

use crate::{
    discover::server::Server,
    eth::eth_message::EthMessage,
    mev,
    p2p::{peer::PeerType, peer_info::PeerInfo},
    server::{inbound_connections::InboundConnections, peers::PEERS},
    token::tokens_to_buy::{get_token_by_address, remove_all_tokens_to_buy},
    utils::wei_gwei_converter::MIN_GAS_PRICE,
    wallets::local_wallets::{generate_rlp_prep_tx, generate_rlp_snappy_prep_tx},
};

pub fn run_local_server(
    disc_server: Option<Arc<Server>>,
    incoming_listener: Arc<InboundConnections>,
    tx_sender: tokio::sync::broadcast::Sender<EthMessage>,
) {
    let disc_server_toggler = disc_server.clone();
    let disc_server_enodes = disc_server.clone();
    tokio::task::spawn(async move {
        //TODO: extract this into at least separate function (and maybe even file)
        let prep = warp::path!("prep" / String).and_then({
            move |token_address: String| {
                let tx_sender = tx_sender.clone();

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
                    // let prep_tx = EthMessage::new_compressed_tx_message(
                    //     generate_rlp_snappy_prep_tx(token, MIN_GAS_PRICE).await,
                    // );
                    // let _ = tx_sender.send(prep_tx);
                    let prep_tx = generate_rlp_prep_tx(token, MIN_GAS_PRICE).await.0;
                    match mev::puissant::send_mev(prep_tx, 1, 60).await {
                        Ok(resp) => {
                            println!("Puissant response: {}", resp);
                            let mut cnt = 0;
                            let mut status_resp = None;
                            while cnt < 5 {
                                match mev::puissant::get_mev_status(&resp.result).await {
                                    Ok(status) => {
                                        status_resp = Some(status);
                                    }
                                    Err(e) => {
                                        cnt += 1;
                                        println!("Puissant status err: {}", e);
                                        tokio::time::sleep(tokio::time::Duration::from_secs(1))
                                            .await;
                                    }
                                }
                            }

                            if let Some(status) = status_resp {
                                println!("Puissant status: {}", status);
                            } else {
                                println!("Puissant status not found");
                            }
                        }
                        Err(e) => {
                            println!("Puissant err: {}", e);
                        }
                    }

                    // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    // cprintln!(
                    //     "<yellow>[{}]Prep sent successfully: {}</>",
                    //     PEERS.len(),
                    //     token.buy_token_address
                    // );
                    return Ok(format!(
                        "Prep sent successfully: {}",
                        token.buy_token_address
                    ));
                }
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
                "Total: {}, Inbound: {}, Outbound: {}\n",
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
            if let Some(disc) = &disc_server_toggler {
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

        let peer_infos = warp::path!("peerlist").and(end()).map(|| {
            let peers = PEERS
                .iter()
                .map(|p| p.value().clone())
                .collect::<Vec<PeerInfo>>();
            PeerInfo::slice_to_json(&peers).unwrap()
        });

        let get_enodes = warp::path("enodes").and(end()).map(move || {
            if let Some(disc) = &disc_server_enodes {
                let enodes = disc.get_bsc_node_enodes();
                return enodes
                    .iter()
                    .map(|e| format!("\"{}\"", e))
                    .collect::<Vec<_>>()
                    .join(",\n");
            } else {
                return "Discovery server not found".to_string();
            }
        });

        let routes = prep
            .or(peer_count)
            .or(refresh_tokens)
            .or(disc)
            .or(peer_infos)
            .or(get_enodes);
        warp::serve(routes).run(([0, 0, 0, 0], 6060)).await;
    });
}

#[derive(Debug, Display)]
enum LocalServerErr {
    InvalidTokenAddress,
    TokenNotFound,
}

impl Reject for LocalServerErr {}
