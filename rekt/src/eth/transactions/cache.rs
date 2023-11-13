use crate::types::hash::H256;

pub static mut CACHE: Vec<u8> = Vec::new();
pub const TX_FETCHED_FLAG: u8 = u8::MAX - 100;

#[derive(Debug, PartialEq)]
pub enum TxCacheStatus {
    Fetched,
    NotFetched,
    FetchedOrAnnounced,
    NotAnnounced,
}

pub fn init_cache() {
    unsafe {
        CACHE.reserve_exact(u32::MAX as usize);
        for _ in 0..u32::MAX {
            CACHE.push(0);
        }
        println!("Tx cache initialized");
    }
}

pub fn mark_as_fetched(hash: &H256) -> TxCacheStatus {
    unsafe {
        let index = convert_hash_to_index(hash);
        if CACHE[index] >= TX_FETCHED_FLAG {
            return TxCacheStatus::Fetched;
        }
        CACHE[index] = TX_FETCHED_FLAG;
        TxCacheStatus::NotFetched
    }
}

pub fn mark_as_announced(hash: &H256) -> TxCacheStatus {
    unsafe {
        let index = convert_hash_to_index(hash);
        if CACHE[index] > 0 {
            return TxCacheStatus::FetchedOrAnnounced;
        }

        CACHE[index] += 1;
        return TxCacheStatus::NotAnnounced;
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
