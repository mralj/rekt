use std::{net::IpAddr, str::FromStr, sync::Arc};

use color_print::cprintln;
use derive_more::Display;
use ethers::types::Address;
use warp::{filters::path::end, reject::Reject, Filter};

use crate::{
    discover::server::Server,
    eth::eth_message::EthMessage,
    our_nodes::add_our_node,
    p2p::{
        peer::{PeerType, END},
        peer_info::PeerInfo,
        Peer,
    },
    server::{inbound_connections::InboundConnections, peers::PEERS},
    token::tokens_to_buy::{get_token_by_address, remove_all_tokens_to_buy},
    utils::wei_gwei_converter::MIN_GAS_PRICE,
    wallets::local_wallets::generate_and_rlp_encode_prep_tx,
};

pub fn run_local_server(
    disc_server: Option<Arc<Server>>,
    incoming_listener: Arc<InboundConnections>,
    this_node_public_ip: Option<IpAddr>,
    peer_tx_tx: tokio::sync::broadcast::Sender<EthMessage>,
) {
    let disc_server_toggler = disc_server.clone();
    let disc_server_enodes = disc_server.clone();
    tokio::task::spawn(async move {
        //TODO: extract this into at least separate function (and maybe even file)
        let prep = warp::path!("prep" / String).and_then({
            move |token_address: String| {
                let peer_tx_tx = peer_tx_tx.clone();
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
                    let prep_tx = EthMessage::new_compressed_tx_message(
                        generate_and_rlp_encode_prep_tx(token, MIN_GAS_PRICE).await,
                    );
                    let start = chrono::Utc::now().timestamp_micros();
                    if peer_tx_tx.send(prep_tx).is_ok() {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        let duration = unsafe { END - start };
                        cprintln!(
                            "<yellow>[{}]Prep sent successfully: {} in {duration}</>",
                            PEERS.len(),
                            token.buy_token_address
                        );
                        return Ok(format!(
                            "Prep sent successfully: {}",
                            token.buy_token_address
                        ));
                    } else {
                        cprintln!("<red> Pep Broadcast failed");
                        return Ok(format!("Prep broadcast failed"));
                    }
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

        let add_our_node =
            warp::path!("addournode" / String)
                .and(end())
                .map(move |node: String| {
                    if let Ok(ip) = IpAddr::from_str(&node) {
                        if let Some(this_node_public_ip) = this_node_public_ip {
                            if ip != this_node_public_ip {
                                add_our_node(format!("{}:6070", node));
                                println!("Added node: {}", node);
                                format!("Added node: {}\n", node)
                            } else {
                                println!("Can't add our own node: {}", node);
                                format!("Can't add our own node: {}\n", node)
                            }
                        } else {
                            add_our_node(format!("{}:6070", node));
                            println!("Added node: {}", node);
                            format!("Added node: {}\n", node)
                        }
                    } else {
                        println!("Failed to add node: {}", node);
                        format!("Failed to add node: {}\n", node)
                    }
                });

        let routes = prep
            .or(peer_count)
            .or(refresh_tokens)
            .or(disc)
            .or(peer_infos)
            .or(get_enodes)
            .or(add_our_node);
        warp::serve(routes).run(([0, 0, 0, 0], 6060)).await;
    });
}

#[derive(Debug, Display)]
enum LocalServerErr {
    InvalidTokenAddress,
    TokenNotFound,
}

impl Reject for LocalServerErr {}
