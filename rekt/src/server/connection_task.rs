use std::time::{Duration, Instant};

use crate::types::node_record::NodeRecord;

#[derive(Debug, Clone)]
pub struct ConnectionTask {
    pub node: NodeRecord,
    pub last_attempt: Instant,
    pub next_attempt: Instant,
    pub attempts: u16,
}

impl ConnectionTask {
    pub fn new(enode: &str) -> Self {
        Self {
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
        this.next_attempt = Instant::now() + Duration::from_secs(35);

        this
    }
}

impl From<NodeRecord> for ConnectionTask {
    fn from(node: NodeRecord) -> Self {
        Self {
            node,
            last_attempt: Instant::now(),
            next_attempt: Instant::now(),
            attempts: 1,
        }
    }
}
