use crate::types::hash::H256;

pub(super) static mut CACHE: Vec<u8> = Vec::new();
pub(super) const ALREADY_FETCHED_MARKER: u8 = 100;
pub(super) const MAX_REQUEST_COUNT: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
pub enum TxCacheStatus {
    NotRequested,
    Requested,
    Fetched,
    NotFetched,
}

pub fn init_cache() {
    unsafe {
        let cache_size = u32::MAX as usize + 1;
        CACHE.reserve_exact(cache_size);
        for _ in 0..cache_size {
            CACHE.push(0);
        }
        println!("Tx cache initialized");
    }
}

pub fn mark_as_fetched(hash: &H256) -> TxCacheStatus {
    unsafe {
        let index = convert_hash_to_index(hash);
        if CACHE[index] >= ALREADY_FETCHED_MARKER {
            TxCacheStatus::Fetched
        } else {
            CACHE[index] = ALREADY_FETCHED_MARKER;
            TxCacheStatus::NotFetched
        }
    }
}

pub fn mark_as_requested(hash: &H256) -> TxCacheStatus {
    unsafe {
        let index = convert_hash_to_index(hash);
        CACHE[index] += 1;
        if CACHE[index] > MAX_REQUEST_COUNT {
            TxCacheStatus::Requested
        } else {
            TxCacheStatus::NotRequested
        }
    }
}

#[inline(always)]
fn convert_hash_to_index(hash: &H256) -> usize {
    unsafe {
        // This is safe because we're absolutely sure that `hash` has at least 4 bytes.
        let bytes = *(&hash[..4] as *const [u8] as *const [u8; 4]);
        u32::from_ne_bytes(bytes) as usize
    }
}
