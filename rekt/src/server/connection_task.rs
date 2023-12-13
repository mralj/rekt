use std::time::{Duration, Instant};

use secp256k1::{PublicKey, SecretKey};

use crate::{cli::Cli, types::node_record::NodeRecord};

#[derive(Debug, Clone)]
pub struct ConnectionTask {
    pub node: NodeRecord,
    pub our_sk: SecretKey,
    pub our_pk: PublicKey,

    pub server_info: Cli,

    pub last_attempt: Instant,
    pub next_attempt: Instant,
    pub attempts: u16,
}

impl ConnectionTask {
    pub fn new(enode: &str, our_pk: PublicKey, our_sk: SecretKey, server_info: Cli) -> Self {
        Self {
            our_pk,
            our_sk,
            server_info,
            node: enode.parse().unwrap(),
            last_attempt: Instant::now(),
            next_attempt: Instant::now(),
            attempts: 1,
        }
    }

    pub fn next_attempt(&self) -> Self {
        let mut this = self.clone();

        this.last_attempt = Instant::now();
        this.attempts += 1;
        //TODO: should we some kind of backoff/smarter logic here?
        this.next_attempt = Instant::now() + Duration::from_secs(20);

        this
    }
}
