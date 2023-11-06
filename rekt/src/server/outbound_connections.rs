use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashSet;
use kanal::{AsyncReceiver, AsyncSender};
use secp256k1::{PublicKey, SecretKey};

use crate::p2p::errors::P2PError;
use crate::p2p::peer::BUY_IS_IN_PROGRESS;
use crate::p2p::DisconnectReason;
use crate::rlpx::RLPXSessionError;
use crate::types::hash::H512;

use super::active_peer_session::connect_to_node;
use super::connection_task::ConnectionTask;
use super::errors::ConnectionTaskError;
use super::peers::BLACKLIST_PEERS_BY_ID;

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

    peer_err: DashSet<H512>,
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
            peer_err: DashSet::with_capacity(1000),
        }
    }

    pub async fn start(self: Arc<Self>) {
        let task_runner = self.clone();
        let retry_runner = self.clone();

        tokio::task::spawn(async move { task_runner.run().await });

        tokio::task::spawn(async move {
            retry_runner.run_retirer().await;
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
            unsafe {
                if BUY_IS_IN_PROGRESS {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
            if task_r.is_err() {
                continue;
            }
            let task = task_r.unwrap();
            match task.err {
                RLPXSessionError::TcpError(_) => {}
                RLPXSessionError::DisconnectRequested(reason) => match reason {
                    DisconnectReason::TooManyPeers => {}
                    _ => {
                        tracing::info!("{}", task.err);
                    }
                },
                RLPXSessionError::P2PError(err) => match err {
                    P2PError::AlreadyConnected | P2PError::AlreadyConnectedToSameIp => {}
                    P2PError::DisconnectRequested(DisconnectReason::TooManyPeers) => {}
                    P2PError::DisconnectRequested(DisconnectReason::DisconnectRequested) => {}
                    P2PError::DisconnectRequested(DisconnectReason::UselessPeer) => {
                        self.peer_err.insert(task.conn_task.node.id);
                        tracing::info!("Useless: {}", self.peer_err.len());
                    }
                    _ => {
                        tracing::info!("{}", task.err);
                    }
                },
                RLPXSessionError::RlpxError(ref err) => match err {
                    crate::rlpx::RLPXError::InvalidAckData => {}
                    _ => {
                        tracing::info!("{}", task.err);
                    }
                },
                _ => {
                    tracing::info!("{}", task.err);
                }
            }
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
}
