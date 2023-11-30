// use crate::types::hash::H256;
//
// pub(super) static mut CACHE: Vec<bool> = Vec::new();
// pub(super) const ALREADY_FETCHED: bool = true;
// pub(super) const NOT_FETCHED: bool = false;
//
// pub fn init_cache() {
//     unsafe {
//         CACHE.reserve_exact(u32::MAX as usize);
//         for _ in 0..u32::MAX {
//             CACHE.push(false);
//         }
//         println!("Tx cache initialized");
//     }
// }
//
// pub fn mark_as_fetched(hash: &H256) -> bool {
//     unsafe {
//         let index = convert_hash_to_index(hash);
//         if CACHE[index] == ALREADY_FETCHED {
//             ALREADY_FETCHED
//         } else {
//             CACHE[index] = ALREADY_FETCHED;
//             NOT_FETCHED
//         }
//     }
// }
//
// pub fn was_fetched(hash: &H256) -> bool {
//     unsafe {
//         let index = convert_hash_to_index(hash);
//         CACHE[index] == ALREADY_FETCHED
//     }
// }
//
// #[inline(always)]
// fn convert_hash_to_index(hash: &H256) -> usize {
//     unsafe {
//         // This is safe because we're absolutely sure that `hash` has at least 4 bytes.
//         let bytes = *(&hash[..4] as *const [u8] as *const [u8; 4]);
//         u32::from_ne_bytes(bytes) as usize
//     }
// }

use dashmap::{mapref::entry::Entry, DashMap};
use static_init::dynamic;

use crate::types::hash::H256;

pub(super) enum TxFetchStatus {
    Fetched,
    Requested(u8),
}

#[dynamic]
static CACHE: DashMap<H256, TxFetchStatus> = DashMap::with_capacity(4_000_000);

pub fn mark_as_fetched(hash: H256) -> bool {
    match CACHE.entry(hash) {
        Entry::Vacant(entry) => {
            entry.insert(TxFetchStatus::Fetched);
            false
        }
        Entry::Occupied(mut entry) => {
            if let TxFetchStatus::Fetched = entry.get() {
                true
            } else {
                entry.insert(TxFetchStatus::Fetched);
                false
            }
        }
    }
}

pub fn mark_as_requested(hash: H256) -> bool {
    match CACHE.entry(hash) {
        Entry::Vacant(entry) => {
            entry.insert(TxFetchStatus::Requested(1));
            false
        }
        Entry::Occupied(mut entry) => match entry.get_mut() {
            TxFetchStatus::Fetched => true,
            TxFetchStatus::Requested(count) => {
                *count += 1;
                if *count > 2 {
                    true
                } else {
                    false
                }
            }
        },
    }
}
