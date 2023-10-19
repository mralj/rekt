use std::hash::{BuildHasher, Hasher};

use dashmap::DashMap;
use static_init::dynamic;

use crate::types::hash::H256;

pub enum TxFetchStatus {
    Fetched,
    InProgress(u8),
}

impl TxFetchStatus {
    pub fn is_fetched(&self) -> bool {
        match self {
            TxFetchStatus::Fetched => true,
            _ => false,
        }
    }

    pub fn should_send_request(&mut self) -> bool {
        if let TxFetchStatus::InProgress(n) = self {
            *n += 1;
            *n < 4
        } else {
            false
        }
    }
}

#[dynamic]
pub static CACHE: DashMap<H256, TxFetchStatus, TxHasherBuilder> =
    DashMap::with_capacity_and_hasher(4_000_000, TxHasherBuilder::default());

//NOTE: this is basically and "Identity hasher" (f(x)= x)
// This is ok since we are storing ETH TX hashes, which are already
// hashed using Keccak256 (SHA3) which produces uniformly distributed values
#[derive(Default)]
pub struct TxHashHasher(u64);
impl Hasher for TxHashHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // Take the first 8 bytes from the input and interpret them as a u64.
        // This will panic if bytes is less than 8 bytes long.
        let bytes: [u8; 8] = bytes[..8]
            .try_into()
            .expect("Should've had at least 8 bytes");
        self.0 = u64::from_ne_bytes(bytes);
    }
}

#[derive(Default, Clone)]
pub struct TxHasherBuilder;
impl BuildHasher for TxHasherBuilder {
    type Hasher = TxHashHasher;

    fn build_hasher(&self) -> Self::Hasher {
        TxHashHasher(0)
    }
}
