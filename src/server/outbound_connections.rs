use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use kanal::{AsyncReceiver, AsyncSender};
use secp256k1::{PublicKey, SecretKey};
use tokio::select;
use tokio::sync::Semaphore;
use tokio::time::interval;

use crate::p2p::DisconnectReason;
use crate::rlpx::{connect_to_node, PeerErr, RLPXSessionError};
use crate::types::hash::H512;

use super::connection_task::ConnectionTask;

const ALWAYS_SLEEP_LITTLE_BIT_MORE_BEFORE_RETRYING_TASK: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct OutboundConnections {
    nodes: Vec<String>,
    our_pub_key: PublicKey,
    our_private_key: secp256k1::SecretKey,

    concurrent_conn_attempts_semaphore: Arc<Semaphore>,

    peers: Arc<DashMap<H512, String>>,
    peers_blacklist_by_id: Arc<DashSet<H512>>,
    peers_blacklist_by_ip: Arc<DashSet<String>>,

    conn_rx: AsyncReceiver<ConnectionTask>,
    conn_tx: AsyncSender<ConnectionTask>,

    retry_rx: AsyncReceiver<PeerErr>,
    retry_tx: AsyncSender<PeerErr>,
}

impl OutboundConnections {
    pub fn new(nodes: Vec<String>) -> Self {
        let our_private_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let our_pub_key =
            secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &our_private_key);

        let semaphore = Arc::new(Semaphore::new(1000));

        let (conn_tx, conn_rx) = kanal::unbounded_async();
        let (retry_tx, retry_rx) = kanal::unbounded_async();

        let peers = Arc::new(DashMap::with_capacity(2 * nodes.len()));
        let peers_blacklist_by_id = Arc::new(DashSet::with_capacity(nodes.len()));
        let peers_blacklist_by_ip = Arc::new(DashSet::with_capacity(nodes.len()));

        Self {
            nodes,
            peers,
            peers_blacklist_by_id,
            peers_blacklist_by_ip,
            our_pub_key,
            our_private_key,
            conn_rx,
            conn_tx,
            retry_rx,
            retry_tx,
            concurrent_conn_attempts_semaphore: semaphore,
        }
    }

    pub async fn start(&self) {
        let task_runner = self.clone();
        let retry_runner = self.clone();
        let log_runner = self.clone();

        tokio::task::spawn(async move {
            task_runner.run().await;
        });

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

    pub async fn run(&self) {
        loop {
            let task = self.conn_rx.recv().await;
            if let Ok(task) = task {
                let we_should_not_try_connecting_to_this_node =
                    self.peers.contains_key(&task.node.id)
                        || self.peers_blacklist_by_id.contains(&task.node.id)
                        || self
                            .peers_blacklist_by_ip
                            .contains(&task.node.address.to_string());

                if we_should_not_try_connecting_to_this_node {
                    continue;
                }

                connect_to_node(
                    task,
                    self.our_private_key,
                    self.our_pub_key,
                    self.concurrent_conn_attempts_semaphore.clone(),
                    self.peers.clone(),
                    self.peers_blacklist_by_ip.clone(),
                    self.retry_tx.clone(),
                );
            }
        }
    }

    pub async fn run_retirer(self) {
        loop {
            let task_r = self.retry_rx.recv().await;
            if task_r.is_err() {
                continue;
            }
            let task = task_r.unwrap();
            let _error_worth_retrying = match task.err {
                RLPXSessionError::DisconnectRequested(reason) => match reason {
                    DisconnectReason::TooManyPeers | DisconnectReason::PingTimeout => reason,
                    _ => continue,
                },
                _ => continue,
            };

            let task = task.conn_task;
            if self.peers_blacklist_by_id.contains(&task.node.id) {
                continue;
            }

            if self
                .peers_blacklist_by_ip
                .contains(&task.node.address.to_string())
            {
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

            let _ = self.conn_tx.send(task).await;
        }
    }

    pub async fn run_logger(&self) {
        let mut count_interval = interval(Duration::from_secs(30));
        let mut info_interval = interval(Duration::from_secs(10 * 60));

        loop {
            select! {
                _ = count_interval.tick() => {
                    println!("{}", self.peers.len());
                },
                _ = info_interval.tick() => {
                    tracing::info!("==================== ==========================  ==========");
                    for v in self.peers.iter() {
                        tracing::info!("{}", v.value())
                    }
                }
            }
        }
    }
}
