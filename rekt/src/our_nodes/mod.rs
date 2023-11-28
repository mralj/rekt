use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use futures::future::join_all;
use tokio::net::UdpSocket;

use crate::{
    p2p::{
        peer::{is_buy_in_progress, BUY_IS_IN_PROGRESS},
        Peer,
    },
    token::{
        token::TokenAddress,
        tokens_to_buy::{get_token_by_address, get_token_to_buy_by_address, mark_token_as_bought},
    },
};

static mut OUR_NODES: Vec<String> = Vec::new();

pub fn add_node(node: String) {
    unsafe {
        OUR_NODES.push(node);
    }
}

pub async fn send_liq_added_signal_to_our_other_nodes(token_address: TokenAddress, gas_price: u64) {
    let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        6070,
    )))
    .await;

    if socket.is_err() {
        println!("Error binding to socket for intra node sending");
        return;
    }

    let socket = socket.unwrap();

    let mut buf = vec![0; 28];
    buf.extend_from_slice(&token_address.as_bytes());
    buf.extend_from_slice(&gas_price.to_be_bytes());

    let mut tasks = Vec::with_capacity(50);
    for node in unsafe { OUR_NODES.iter() } {
        tasks.push(socket.send_to(&buf, node));
    }

    join_all(tasks).await;
}

pub fn listen_on_liq_added_signal() {
    tokio::spawn(async move {
        let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            6070,
        )))
        .await;

        if socket.is_err() {
            println!("Error binding to socket for intra node listening");
            return;
        }

        let socket = socket.unwrap();
        let mut buf = vec![0; 1024];

        loop {
            if is_buy_in_progress() {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }

            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    if len != 28 {
                        println!("Received invalid liq added signal from {}", addr);
                        continue;
                    }

                    unsafe {
                        BUY_IS_IN_PROGRESS = true;
                    }

                    let token = TokenAddress::from_slice(&buf[0..20]);

                    let gas = {
                        let mut gas_bytes = [0u8; 8];
                        gas_bytes.copy_from_slice(&buf[20..28]);
                        u64::from_be_bytes(gas_bytes) // Big endian to match to_bytes
                    };

                    println!(
                        "Received liq added signal from {} for token {:?} with gas price {}",
                        addr,
                        TokenAddress::from(token),
                        gas
                    );

                    if let Some(mut token) = get_token_to_buy_by_address(&token) {
                        if let Some(buy_tx) = token.get_buy_txs(gas) {
                            let peer_count = Peer::send_tx(buy_tx).await;
                            println!("Sent buy tx to {} peers", peer_count);
                            mark_token_as_bought(token.buy_token_address);
                        } else {
                            println!("No buy txs found for token {:?}", token.buy_token_address);
                        }
                    } else {
                        println!("Token not found");
                    }

                    unsafe {
                        BUY_IS_IN_PROGRESS = false;
                    }
                }
                Err(e) => {
                    println!("Error receiving from intra node socket: {}", e);
                }
            }
        }
    });
}
