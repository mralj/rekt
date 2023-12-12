use std::time::{Duration, Instant};

use secp256k1::{PublicKey, SecretKey};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::p2p::peer::is_buy_in_progress;
use crate::rlpx::RLPXSessionError;

use super::active_peer_session::connect_to_node;
use super::connection_task::ConnectionTask;
use super::errors::ConnectionTaskError;
use super::peers::{peer_is_blacklisted, BLACKLIST_PEERS_BY_ID};

const ALWAYS_SLEEP_LITTLE_BIT_MORE_BEFORE_RETRYING_TASK: Duration = Duration::from_secs(5);

pub struct OutboundConnections {
    nodes: Vec<String>,
    our_pub_key: PublicKey,
    our_private_key: secp256k1::SecretKey,

    conn_rx: UnboundedReceiver<ConnectionTaskError>,
    conn_tx: UnboundedSender<ConnectionTaskError>,

    cli: crate::cli::Cli,
}

impl OutboundConnections {
    pub fn new(
        our_private_key: SecretKey,
        our_pub_key: PublicKey,
        nodes: Vec<String>,
        conn_rx: UnboundedReceiver<ConnectionTaskError>,
        conn_tx: UnboundedSender<ConnectionTaskError>,
        cli: crate::cli::Cli,
    ) -> Self {
        Self {
            nodes,
            our_pub_key,
            our_private_key,
            conn_rx,
            conn_tx,
            cli,
        }
    }

    pub fn run(mut self) {
        tokio::task::spawn(async move {
            for node in self.nodes.iter() {
                let task = ConnectionTask::new(
                    node,
                    self.our_pub_key,
                    self.our_private_key,
                    self.cli.clone(),
                );

                connect_to_node(task, self.conn_tx.clone());
            }
            loop {
                if let Some(task) = self.conn_rx.recv().await {
                    if let Some(err) = task.err {
                        tracing::error!("{}", err);

                        match err {
                            RLPXSessionError::TcpError(_) => continue,
                            RLPXSessionError::ConnectionClosed => continue,
                            _ => {}
                        }
                    }

                    let task = task.conn_task;

                    if is_buy_in_progress() {
                        tokio::time::sleep(Duration::from_secs(90)).await;
                    }

                    if peer_is_blacklisted(&task.node) {
                        continue;
                    }

                    let it_is_not_yet_time_to_retry = !task
                        .next_attempt
                        .saturating_duration_since(Instant::now())
                        .is_zero();

                    if it_is_not_yet_time_to_retry {
                        tokio::time::sleep(
                            ALWAYS_SLEEP_LITTLE_BIT_MORE_BEFORE_RETRYING_TASK
                                + (task.next_attempt - Instant::now()),
                        )
                        .await;
                    }

                    if BLACKLIST_PEERS_BY_ID.contains(&task.node.id) {
                        continue;
                    }

                    connect_to_node(task, self.conn_tx.clone());
                }
            }
        });
    }
}
