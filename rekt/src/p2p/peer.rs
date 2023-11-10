use derive_more::Display;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::time::interval;

use color_print::cprintln;
use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::error;

use super::errors::P2PError;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use super::tx_sender::{UnsafeSyncPtr, PEERS_SELL};
use crate::eth;
use crate::eth::eth_message::EthMessage;
use crate::eth::msg_handler::EthMessageHandler;
use crate::eth::status_message::{StatusMessage, UpgradeStatusMessage};
use crate::eth::types::protocol::EthProtocol;
use crate::p2p::p2p_wire::P2PWire;
use crate::rlpx::TcpWire;
use crate::server::peers::{
    blacklist_peer, check_if_already_connected_to_peer, PEERS, PEERS_BY_IP,
};
use crate::token::token::Token;
use crate::token::tokens_to_buy::{mark_token_as_bought, remove_all_tokens_to_buy};
use crate::types::hash::H512;

use crate::types::node_record::NodeRecord;
use crate::utils::helpers::{get_bsc_token_url, get_bsc_tx_url};
use crate::wallets::local_wallets::{
    generate_and_rlp_encode_sell_tx, update_nonces_for_local_wallets,
};

pub static mut BUY_IS_IN_PROGRESS: bool = false;
pub static mut SELL_IS_IN_PROGRESS: bool = false;

pub static mut UNDER_10: usize = 0;
pub static mut UNDER_20: usize = 0;
pub static mut UNDER_30: usize = 0;
pub static mut UNDER_50: usize = 0;
pub static mut UNDER_100: usize = 0;
pub static mut UNDER_200: usize = 0;
pub static mut OVER_200: usize = 0;
pub static mut TOTAL: usize = 0;

pub fn is_buy_in_progress() -> bool {
    unsafe { BUY_IS_IN_PROGRESS }
}

pub fn is_sell_in_progress() -> bool {
    unsafe { SELL_IS_IN_PROGRESS }
}

pub fn is_buy_or_sell_in_progress() -> bool {
    is_buy_in_progress() || is_sell_in_progress()
}

const BLOCK_DURATION_IN_MILLIS: u64 = 3000;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Display)]
pub enum PeerType {
    Inbound,
    Outbound,
}

#[derive(Debug)]
pub struct Peer {
    pub id: H512,
    pub(crate) node_record: NodeRecord,
    pub(crate) info: String,
    pub(crate) peer_type: PeerType,

    pub(super) connection: P2PWire,

    protocol_version: ProtocolVersion,
}

impl Peer {
    pub fn new(
        enode: NodeRecord,
        id: H512,
        protocol: usize,
        info: String,
        connection: TcpWire,
        peer_type: PeerType,
    ) -> Self {
        Self {
            id,
            connection: P2PWire::new(connection),
            info,
            peer_type,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
        }
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.node_record.str, self.id, self.protocol_version
        )
    }
}

impl Peer {
    pub async fn run(&mut self) -> Result<(), P2PError> {
        check_if_already_connected_to_peer(&self.node_record)?;
        if let Err(e) = self.handshake().await {
            blacklist_peer(&self.node_record);
            return Err(e);
        }
        check_if_already_connected_to_peer(&self.node_record)?;

        PEERS.insert(self.node_record.id, PeerInfo::from(self as &Peer));
        PEERS_BY_IP.insert(self.node_record.ip.clone());

        let peer_ptr = UnsafeSyncPtr {
            peer: self as *mut _,
        };
        PEERS_SELL
            .lock()
            .await
            .insert(self.node_record.id, peer_ptr);

        loop {
            let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;
            let received_on = msg.created_on;
            if let Ok(handler_resp) = eth::msg_handler::handle_eth_message(msg) {
                match handler_resp {
                    EthMessageHandler::Response(msg) => {
                        self.connection.send(msg).await?;
                        let elapsed = received_on.elapsed().as_micros();
                        unsafe {
                            TOTAL += 1;
                            if elapsed <= 10 {
                                UNDER_10 += 1;
                            } else if elapsed <= 20 {
                                UNDER_20 += 1;
                            } else if elapsed <= 30 {
                                UNDER_30 += 1;
                            } else if elapsed <= 50 {
                                UNDER_50 += 1;
                            } else if elapsed <= 100 {
                                UNDER_100 += 1;
                            } else if elapsed <= 200 {
                                UNDER_200 += 1;
                            } else {
                                OVER_200 += 1;
                            }
                        }
                    }
                    EthMessageHandler::Buy(mut buy_info) => {
                        if let Some(buy_txs_eth_message) =
                            buy_info.token.get_buy_txs(buy_info.gas_price)
                        {
                            println!("Received BUY in {:?}", received_on.elapsed());
                            let sent_txs_to_peer_count = Peer::send_tx(buy_txs_eth_message).await;
                            println!("Sent BUY in {:?}", received_on.elapsed());

                            mark_token_as_bought(buy_info.token.buy_token_address);
                            unsafe {
                                BUY_IS_IN_PROGRESS = false;
                                SELL_IS_IN_PROGRESS = true;
                            }
                            cprintln!(
                                "<b><green>[{}][{sent_txs_to_peer_count}] Bought token: {}</></>\nliq TX: {} ",
                                buy_info.time.format("%Y-%m-%d %H:%M:%S:%f"),
                                get_bsc_token_url(buy_info.token.buy_token_address),
                                get_bsc_tx_url(buy_info.hash)
                            );

                            Self::sell(buy_info.token).await;
                        }
                    }
                    EthMessageHandler::None => {}
                }
            }
        }
    }

    async fn handshake(&mut self) -> Result<(), P2PError> {
        let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;

        if msg.id != EthProtocol::StatusMsg {
            error!("Expected status message, got {:?}", msg.id);
            return Err(P2PError::ExpectedStatusMessage);
        }

        let status_msg = StatusMessage::decode(&mut &msg.data[..])?;

        if StatusMessage::validate(&status_msg, &self.protocol_version).is_err() {
            return Err(P2PError::CouldNotValidateStatusMessage);
        }

        self.connection
            .send(StatusMessage::get(&self.protocol_version))
            .await?;

        self.handle_upgrade_status_messages().await
    }

    async fn handle_upgrade_status_messages(&mut self) -> Result<(), P2PError> {
        if self.protocol_version == ProtocolVersion::Eth66 {
            return Ok(());
        }

        self.connection.send(UpgradeStatusMessage::get()).await?;
        let msg = self.connection.next().await.ok_or(P2PError::NoMessage)??;
        if msg.id != EthProtocol::UpgradeStatusMsg {
            return Err(P2PError::ExpectedUpgradeStatusMessage);
        }

        Ok(())
    }

    async fn sell(token: Token) {
        //TODO: handle transfer instead of selling scenario
        // sleep so that we don't sell immediately
        tokio::time::sleep(Duration::from_millis(200)).await;
        for i in 0..token.sell_config.sell_count {
            //this is because for the first sell the nonce is up to date with blockchain
            //only after first sell we need to "update it manually"
            let increment_sell_nonce_after_first_sell = i > 0;
            let sell_tx = EthMessage::new_tx_message(
                generate_and_rlp_encode_sell_tx(increment_sell_nonce_after_first_sell).await,
            );

            let count = Peer::send_tx(sell_tx).await;

            cprintln!(
                "<blue>[{count}][{}/{}]Selling token: {:#x}</>",
                i + 1,
                token.sell_config.sell_count,
                token.buy_token_address
            );

            // wait for sell tx to be mined before sending the next one
            // we also wait bit more before sending new tx since our code is super fast 😅
            tokio::time::sleep(Duration::from_millis(BLOCK_DURATION_IN_MILLIS + 500)).await;
        }

        cprintln!(
            "<rgb(255,165,0)>Done selling token: {}</>",
            get_bsc_token_url(token.buy_token_address)
        );

        unsafe {
            SELL_IS_IN_PROGRESS = false;
        }

        // this will refresh token list with proper nonces
        // sleep for a while to make sure public nodes have latest nonces
        tokio::time::sleep(Duration::from_millis(3 * BLOCK_DURATION_IN_MILLIS)).await;
        update_nonces_for_local_wallets().await;
        remove_all_tokens_to_buy();
    }
}

pub fn logger() {
    tokio::spawn(async {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(60),
        ));

        let started = tokio::time::Instant::now();

        while let Some(_) = stream.next().await {
            unsafe {
                println!("=== STATS ===");
                println!("Test duration: {:?} min", started.elapsed().as_secs() / 60);
                println!("TOTAL: {}", TOTAL);
                println!(
                    "UNDER_10: {}, {}%",
                    UNDER_10,
                    f64::round(((UNDER_10 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_20: {}, {}%",
                    UNDER_20,
                    f64::round(((UNDER_20 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_30: {}, {}%",
                    UNDER_30,
                    f64::round(((UNDER_30 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_50: {}, {}%",
                    UNDER_50,
                    f64::round(((UNDER_50 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_100: {}, {}%",
                    UNDER_100,
                    f64::round(((UNDER_100 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "UNDER_200: {}, {}%",
                    UNDER_200,
                    f64::round(((UNDER_200 as f64) * 100.0) / TOTAL as f64)
                );
                println!(
                    "OVER_200: {}, {}%",
                    OVER_200,
                    f64::round(((OVER_200 as f64) * 100.0) / TOTAL as f64)
                );
                println!("=== END ===");
            }
        }
    });
}
