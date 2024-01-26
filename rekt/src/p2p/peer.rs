use derive_more::Display;
use ethers::types::U256;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;

use color_print::cprintln;
use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tracing::error;

use super::errors::P2PError;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use crate::cli::Cli;
use crate::eth::eth_message::EthMessage;
use crate::eth::msg_handler::EthMessageHandler;
use crate::eth::status_message::{StatusMessage, UpgradeStatusMessage};
use crate::eth::transactions::decoder::BuyTokenInfo;
use crate::eth::types::protocol::EthProtocol;
use crate::google_sheets::LogToSheets;
use crate::mev::puissant::ApiResponse;
use crate::p2p::p2p_wire::P2PWire;
use crate::rlpx::TcpWire;
use crate::server::peers::{
    blacklist_peer, check_if_already_connected_to_peer, PEERS, PEERS_BY_IP,
};
use crate::token::tokens_to_buy::{mark_token_as_bought, remove_all_tokens_to_buy};
use crate::types::hash::H512;
use crate::{eth, google_sheets, mev};

use crate::types::node_record::NodeRecord;
use crate::utils::helpers::{get_bsc_token_url, get_bsc_tx_url};
use crate::wallets::local_wallets::{
    generate_and_rlp_encode_sell_tx, generate_mev_buy_tx, update_nonces_for_local_wallets,
    MEV_WALLET,
};

pub static mut BUY_IS_IN_PROGRESS: bool = false;
pub static mut SELL_IS_IN_PROGRESS: bool = false;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Display, Serialize, Deserialize)]
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
    pub(crate) td: u64,

    pub(super) connection: P2PWire,

    pub(super) protocol_version: ProtocolVersion,

    tx_sender: broadcast::Sender<EthMessage>,

    cli: Cli,
}

impl Peer {
    pub fn new(
        enode: NodeRecord,
        id: H512,
        protocol: usize,
        info: String,
        connection: TcpWire,
        peer_type: PeerType,
        cli: Cli,
        tx_sender: broadcast::Sender<EthMessage>,
    ) -> Self {
        Self {
            id,
            cli,
            connection: P2PWire::new(connection),
            info,
            peer_type,
            tx_sender,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
            td: 0,
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

        let (ping_send, mut ping_recv) = tokio::sync::mpsc::channel(1);
        Self::start_pinger(ping_send);

        let mut tx_receiver = self.tx_sender.subscribe();

        loop {
            select! {
                biased;
                tx = tx_receiver.recv() => {
                    if let Ok(tx) = tx {
                        self.connection.send(tx).await?;
                    }
                },
                msg = self.connection.next(), if !is_buy_in_progress() => {
                    let msg = msg.ok_or(P2PError::NoMessage)??;
                    if let Ok(handler_resp) = eth::msg_handler::handle_eth_message(msg) {
                        match handler_resp {
                            EthMessageHandler::None => {},
                            EthMessageHandler::Response(msg) => {
                                self.connection.send(msg).await?;
                            }
                            EthMessageHandler::Buy(mut buy_info) => {
                               let (buy_txs, mev_buy_tx) = buy_info.token.get_buy_txs(buy_info.gas_price);
                               let buy_txs = match buy_txs {
                                                Some(buy_txs) => buy_txs,
                                                None => {
                                                    println!("LIQ has gwei that we haven't prepared txs for, preparing now...");
                                                    let tx = buy_info
                                                             .token
                                                             .prepare_buy_txs_for_gas_price(buy_info.gas_price)
                                                             .await;
                                                    tx
                                                    }
                                            };

                                 let _ = self.tx_sender.send(buy_txs);
                                 let mev_buy_tx = match mev_buy_tx {
                                                Some(mev_buy_tx) => mev_buy_tx,
                                                None => {
                                                     let mev_wallet = &mut MEV_WALLET.write().await;
                                                     let mev_tx = generate_mev_buy_tx(mev_wallet, U256::from(buy_info.gas_price)).await;
                                                     let mev_tx = hex::encode(&mev_tx);
                                                     let mev_tx = format!("0x{}", mev_tx);
                                                     mev_tx
                                                    }
                                            };
                        self.sell(&buy_info, mev_buy_tx).await;
                        if let Err(e) = google_sheets::write_data_to_sheets(
                            LogToSheets::new(&self.cli, &self, &buy_info).await,
                        )
                        .await
                        {
                            error!("Failed to write to sheets: {}", e);
                        }
                            }
                        }
                    }
                },
                ping = ping_recv.recv(), if !is_buy_in_progress() => {
                    if ping.is_some() {
                        self.connection.send(EthMessage::new_devp2p_ping_message()).await?;
                    }
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

        self.td = status_msg.total_difficulty;

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

    async fn sell(&self, buy_info: &BuyTokenInfo, mev_buy_tx: String) {
        //async fn sell(&self, buy_info: &BuyTokenInfo, mev_resp: anyhow::Result<ApiResponse>) {
        // let mev_id = match mev_resp {
        //     Ok(r) => {
        //         println!(
        //             "[{}] Puissant response: {}",
        //             chrono::Utc::now().format("%Y-%m-%d %H:%M:%S:%f"),
        //             r.result
        //         );
        //         Some(r.result)
        //     }
        //     Err(e) => {
        //         println!("Puissant err: {}", e);
        //         None
        //     }
        // };
        //TODO: handle transfer instead of selling scenario
        // sleep so that we don't sell immediately
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _mev_resp = mev::puissant::send_mev(1, 5, &buy_info, mev_buy_tx).await;
        mark_token_as_bought(buy_info.token.buy_token_address);
        unsafe {
            BUY_IS_IN_PROGRESS = false;
            SELL_IS_IN_PROGRESS = true;
        }
        cprintln!(
            "<b><green>[{}]Bought token: {}</></>\nliq TX: {} ",
            buy_info.time.format("%Y-%m-%d %H:%M:%S:%f"),
            get_bsc_token_url(buy_info.token.buy_token_address),
            get_bsc_tx_url(buy_info.hash)
        );

        let token = &buy_info.token;
        for i in 0..token.sell_config.sell_count {
            //this is because for the first sell the nonce is up to date with blockchain
            //only after first sell we need to "update it manually"
            let increment_sell_nonce_after_first_sell = i > 0;
            let sell_tx = EthMessage::new_tx_message(
                generate_and_rlp_encode_sell_tx(increment_sell_nonce_after_first_sell).await,
            );

            let _ = self.tx_sender.send(sell_tx);
            cprintln!(
                "<blue>[{}/{}]Selling token: {:#x}</>",
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

        // if let Some(id) = mev_id {
        //     match mev::puissant::get_mev_status(&id).await {
        //         Ok(status) => {
        //             println!("Puissant status:\n {}", status);
        //         }
        //         Err(e) => {
        //             println!("Puissant status err: {}", e);
        //         }
        //     }
        // }

        // this will refresh token list with proper nonces
        // sleep for a while to make sure public nodes have latest nonces
        tokio::time::sleep(Duration::from_millis(3 * BLOCK_DURATION_IN_MILLIS)).await;
        update_nonces_for_local_wallets().await;
        remove_all_tokens_to_buy();
    }

    pub(crate) fn start_pinger(ping_sender: mpsc::Sender<()>) {
        tokio::spawn(async move {
            let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
                std::time::Duration::from_secs(15), // same as in geth
            ));

            while let Some(_) = stream.next().await {
                if is_buy_in_progress() {
                    continue;
                }

                if let Err(_) = ping_sender.send(()).await {
                    return;
                }
            }
        });
    }
}
