use std::sync::Arc;
use std::time::{Duration, Instant};

use kanal::{AsyncReceiver, AsyncSender};
use secp256k1::{PublicKey, SecretKey};
use tokio::time::interval;

use crate::p2p::errors::P2PError;
use crate::p2p::DisconnectReason;
use crate::rlpx::RLPXSessionError;

use super::active_peer_session::connect_to_node;
use super::connection_task::ConnectionTask;
use super::errors::ConnectionTaskError;
use super::peers::{BLACKLIST_PEERS_BY_ID, PEERS};

const ALWAYS_SLEEP_LITTLE_BIT_MORE_BEFORE_RETRYING_TASK: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct OutboundConnections {
    nodes: Vec<String>,
    our_pub_key: PublicKey,
    our_private_key: secp256k1::SecretKey,

    conn_rx: AsyncReceiver<ConnectionTask>,
    conn_tx: AsyncSender<ConnectionTask>,

    retry_rx: AsyncReceiver<ConnectionTaskError>,
    retry_tx: AsyncSender<ConnectionTaskError>,
}

impl OutboundConnections {
    pub fn new(our_private_key: SecretKey, our_pub_key: PublicKey, nodes: Vec<String>) -> Self {
        let (conn_tx, conn_rx) = kanal::unbounded_async();
        let (retry_tx, retry_rx) = kanal::unbounded_async();

        Self {
            nodes,
            our_pub_key,
            our_private_key,
            conn_rx,
            conn_tx,
            retry_rx,
            retry_tx,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let task_runner = self.clone();
        let retry_runner = self.clone();
        let log_runner = self.clone();

        tokio::task::spawn(async move { task_runner.run().await });

        tokio::task::spawn(async move {
            retry_runner.run_retirer().await;
        });

        tokio::task::spawn(async move {
            log_runner.run_logger().await;
        });

        for node in self.nodes.iter() {
            let task = ConnectionTask::new(node);

            let _ = self.conn_tx.send(task).await;
        }
    }

    async fn run(&self) {
        loop {
            let task = self.conn_rx.recv().await;
            if let Ok(task) = task {
                if BLACKLIST_PEERS_BY_ID.contains(&task.node.id) {
                    continue;
                }

                connect_to_node(
                    task,
                    self.our_private_key,
                    self.our_pub_key,
                    self.retry_tx.clone(),
                );
            }
        }
    }

    async fn run_retirer(&self) {
        loop {
            let task_r = self.retry_rx.recv().await;
            if task_r.is_err() {
                continue;
            }
            let task = task_r.unwrap();
            tracing::info!("{}", task.err);
            match task.err {
                RLPXSessionError::DisconnectRequested(DisconnectReason::TooManyPeers) => {}
                RLPXSessionError::DisconnectRequested(DisconnectReason::PingTimeout) => {}
                RLPXSessionError::P2PError(P2PError::AlreadyConnected) => {}
                RLPXSessionError::P2PError(P2PError::AlreadyConnectedToSameIp) => {}
                _ => continue,
            };

            let task = task.conn_task;
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

            let _ = self.conn_tx.send(task).await;
        }
    }

    async fn run_logger(&self) {
        let mut info_interval = interval(Duration::from_secs(5 * 60));

        loop {
            info_interval.tick().await;
            tracing::info!("==================== ==========================  ==========");
            tracing::info!("PEER COUNT: {}", PEERS.len());
        }
    }
}
