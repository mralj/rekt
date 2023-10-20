use std::fmt::{Display, Formatter};
use std::time::Duration;

use color_print::cprintln;
use futures::{SinkExt, StreamExt};

use open_fastrlp::Decodable;
use tokio::select;
use tokio::sync::broadcast;
use tracing::error;

use super::errors::P2PError;
use super::peer_info::PeerInfo;
use super::protocol::ProtocolVersion;
use crate::eth;
use crate::eth::eth_message::EthMessage;
use crate::eth::msg_handler::EthMessageHandler;
use crate::eth::status_message::{StatusMessage, UpgradeStatusMessage};
use crate::eth::types::protocol::EthProtocol;
use crate::p2p::p2p_wire::P2PWire;
use crate::rlpx::TcpWire;
use crate::server::peers::{check_if_already_connected_to_peer, PEERS, PEERS_BY_IP};
use crate::token::token::Token;
use crate::token::tokens_to_buy::{mark_token_as_bought, remove_all_tokens_to_buy};
use crate::types::hash::H512;

use crate::types::node_record::NodeRecord;
use crate::utils::helpers::{get_bsc_token_url, get_bsc_tx_url};
use crate::wallets::local_wallets::{
    generate_and_rlp_encode_sell_tx, update_nonces_for_local_wallets,
};

pub static mut BUY_IS_IN_PROGRESS: bool = false;
const BLOCK_DURATION_IN_SECS: u64 = 3;

#[derive(Debug)]
pub struct Peer {
    pub id: H512,
    pub(crate) node_record: NodeRecord,
    pub(crate) info: String,

    protocol_version: ProtocolVersion,

    connection: P2PWire,

    send_txs_channel: broadcast::Sender<EthMessage>,
}

impl Peer {
    pub fn new(
        enode: NodeRecord,
        id: H512,
        protocol: usize,
        info: String,
        connection: TcpWire,
        send_txs_channel: broadcast::Sender<EthMessage>,
    ) -> Self {
        Self {
            id,
            connection: P2PWire::new(connection),
            info,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
            send_txs_channel,
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
        self.handshake().await?;
        check_if_already_connected_to_peer(&self.node_record)?;

        PEERS.insert(self.node_record.id, PeerInfo::from(self as &Peer));
        PEERS_BY_IP.insert(self.node_record.ip.clone());

        let mut txs_to_send_receiver = self.send_txs_channel.subscribe();
        loop {
            select! {
                biased;
                msg_to_send = txs_to_send_receiver.recv() => {
                    if let Ok(msg) = msg_to_send {
                        self.connection.send(msg).await?;
                    }
                },
                msg = self.connection.next(), if unsafe {!BUY_IS_IN_PROGRESS} => {
                    let msg = msg.ok_or(P2PError::NoMessage)??;
                    if let Ok(handler_resp) = eth::msg_handler::handle_eth_message(msg) {
                        match handler_resp {
                            EthMessageHandler::Response(msg) => {
                                self.connection.send(msg).await?;
                            },
                            EthMessageHandler::Buy(mut buy_info) => {
                                if let Some(buy_txs_eth_message) = buy_info.token.get_buy_txs(buy_info.gas_price) {
                                    let _ = self.send_txs_channel.send(buy_txs_eth_message);

                                    //TODO: handle this properly
                                    // probably I'll use Barrier to wait for all txs to be sent
                                    mark_token_as_bought(buy_info.token.buy_token_address);
                                    cprintln!("<b><green>Bought token: {}</></>\nliq TX: {} ",
                                              get_bsc_token_url(buy_info.token.buy_token_address),
                                              get_bsc_tx_url(buy_info.hash));
                                    unsafe {
                                        BUY_IS_IN_PROGRESS = false;
                                    }

                                    Self::sell(buy_info.token, self.send_txs_channel.clone());
                                }
                            },
                            EthMessageHandler::None => {},
                        }
                    }
                },
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

    fn sell(token: Token, send_txs_channel: broadcast::Sender<EthMessage>) {
        //TODO: handle transfer instead of selling scenario
        tokio::spawn(async move {
            // sleep so that we don't sell immediately
            tokio::time::sleep(Duration::from_millis(200)).await;
            for i in 0..token.sell_config.sell_count {
                //this is because for the first sell the nonce is up to date with blockchain
                //only after first sell we need to "update it manually"
                let increment_sell_nonce_after_first_sell = i > 0;
                let sell_tx = EthMessage::new_tx_message(
                    generate_and_rlp_encode_sell_tx(increment_sell_nonce_after_first_sell).await,
                );

                match send_txs_channel.send(sell_tx) {
                    Ok(_) => {
                        cprintln!(
                            "<blue>[{}/{}]Selling token: {:#x}</>",
                            i + 1,
                            token.sell_config.sell_count,
                            token.buy_token_address
                        );
                    }
                    Err(e) => {
                        cprintln!("<red> Channel error: {e}</>");
                    }
                }
                // wait for sell tx to be mined before sending the next one
                tokio::time::sleep(Duration::from_secs(BLOCK_DURATION_IN_SECS)).await;
            }

            cprintln!(
                "Done selling token: {}",
                get_bsc_token_url(token.buy_token_address)
            );

            // this will refresh token list with proper nonces
            // sleep for a while to make sure public nodes have latest nonces
            tokio::time::sleep(Duration::from_secs(3 * BLOCK_DURATION_IN_SECS)).await;
            update_nonces_for_local_wallets().await;
            remove_all_tokens_to_buy();
        });
    }
}
