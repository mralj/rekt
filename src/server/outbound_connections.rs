use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
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

    semaphore: Arc<Semaphore>,
    peers: Arc<DashMap<H512, String>>,

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

        Self {
            nodes,
            peers,
            our_pub_key,
            our_private_key,
            semaphore,
            conn_rx,
            conn_tx,
            retry_rx,
            retry_tx,
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
                connect_to_node(
                    task,
                    self.our_private_key,
                    self.our_pub_key,
                    self.semaphore.clone(),
                    self.peers.clone(),
                    self.retry_tx.clone(),
                );
            }
        }
    }

    pub async fn run_retirer(&self) {
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

            if task
                .conn_task
                .next_attempt
                .saturating_duration_since(Instant::now())
                .is_zero()
            {
                let _ = self.conn_tx.send(task.conn_task).await;
                continue;
            }

            tokio::time::sleep(
                ALWAYS_SLEEP_LITTLE_BIT_MORE_BEFORE_RETRYING_TASK
                    + (task.conn_task.next_attempt - Instant::now()),
            )
            .await;
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
