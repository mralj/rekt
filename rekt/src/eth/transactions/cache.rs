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
    }
}

pub fn mark_as_fetched(hash: &H256) -> TxCacheStatus {
    unsafe {
        let index = convert_hash_to_index(hash);
        if index >= CACHE.len() - 1 {
            panic!("Index out of bounds");
        }
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
        if index >= CACHE.len() - 1 {
            panic!("Index out of bounds");
        }

        if CACHE[index] > 0 {
            return TxCacheStatus::FetchedOrAnnounced;
        }

        CACHE[index] += 1;
        return TxCacheStatus::NotAnnounced;
    }
}

#[inline(always)]
fn convert_hash_to_index(hash: &H256) -> usize {
    let bytes: [u8; 4] = hash[..4]
        .try_into()
        .expect("Should've had at least 4 bytes");
    let index = u32::from_ne_bytes(bytes) as usize;
    index
}
