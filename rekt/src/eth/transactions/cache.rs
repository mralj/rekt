use std::hash::{BuildHasher, Hasher};

use dashmap::DashMap;
use static_init::dynamic;

use crate::types::hash::H256;

pub static mut CACHE: Vec<bool> = Vec::new();

pub fn init_cache() {
    unsafe {
        CACHE = vec![false; u32::MAX as usize];
    }
}

pub fn insert(hash: &H256) -> bool {
    unsafe {
        if CACHE[convert_hash_to_index(hash)] {
            return true;
        }
        CACHE[convert_hash_to_index(hash)] = true;
        false
    }
}

pub fn has(hash: &H256) -> bool {
    unsafe { CACHE[convert_hash_to_index(hash)] }
}

#[inline(always)]
fn convert_hash_to_index(hash: &H256) -> usize {
    let bytes: [u8; 4] = hash[..4]
        .try_into()
        .expect("Should've had at least 8 bytes");
    let index = u32::from_ne_bytes(bytes) as usize;
    index
}

// #[dynamic]
// pub static CACHE: DashMap<H256, (), TxHasherBuilder> =
//     DashMap::with_capacity_and_hasher(4_000_000, TxHasherBuilder::default());
//
// //NOTE: this is basically and "Identity hasher" (f(x)= x)
// // This is ok since we are storing ETH TX hashes, which are already
// // hashed using Keccak256 (SHA3) which produces uniformly distributed values
// #[derive(Default)]
// pub struct TxHashHasher(u64);
// impl Hasher for TxHashHasher {
//     fn finish(&self) -> u64 {
//         self.0
//     }
//
//     fn write(&mut self, bytes: &[u8]) {
//         // Take the first 8 bytes from the input and interpret them as a u64.
//         // This will panic if bytes is less than 8 bytes long.
//         let bytes: [u8; 8] = bytes[..8]
//             .try_into()
//             .expect("Should've had at least 8 bytes");
//         self.0 = u64::from_ne_bytes(bytes);
//     }
// }
//
// #[derive(Default, Clone)]
// pub struct TxHasherBuilder;
// impl BuildHasher for TxHasherBuilder {
//     type Hasher = TxHashHasher;
//
//     fn build_hasher(&self) -> Self::Hasher {
//         TxHashHasher(0)
//     }
// }
