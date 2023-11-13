use bytes::Bytes;
use std::hash::Hasher;
use twox_hash::XxHash32;

pub(super) const ALREADY_CACHED: bool = true;
pub(super) const NOT_CACHED: bool = false;

pub(super) static mut CACHE_TXS: Vec<bool> = Vec::new();
pub(super) static mut CACHE_HASHES: Vec<bool> = Vec::new();

pub fn init_cache() {
    unsafe {
        CACHE_TXS.reserve_exact(u32::MAX as usize);
        for _ in 0..u32::MAX {
            CACHE_TXS.push(false);
        }

        CACHE_HASHES.reserve_exact(u32::MAX as usize);
        for _ in 0..u32::MAX {
            CACHE_HASHES.push(false);
        }
    }
    println!("P2P wire cache initialized");
}

pub(super) fn insert_hash(data: &Bytes) -> bool {
    let index = hash(data);
    if index >= u32::MAX as usize {
        println!("index out of range");
        return false;
    }
    unsafe {
        if CACHE_HASHES[index] == true {
            return ALREADY_CACHED;
        }
        CACHE_HASHES[index] = true;
    }

    NOT_CACHED
}

pub(super) fn insert_tx(data: &Bytes) -> bool {
    let index = hash(data);
    if index >= u32::MAX as usize {
        println!("index out of range");
        return false;
    }
    unsafe {
        if CACHE_TXS[index] == true {
            return ALREADY_CACHED;
        }
        CACHE_TXS[index] = true;
    }

    NOT_CACHED
}

#[inline(always)]
fn hash(data: &Bytes) -> usize {
    let start = tokio::time::Instant::now();

    let mut hasher = XxHash32::default();
    hasher.write(data);
    let h = hasher.finish() as usize;
    println!("Elapsed: {:?}", start.elapsed());

    h
}
