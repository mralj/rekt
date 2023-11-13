use crate::types::hash::H256;

pub static mut CACHE: Vec<bool> = Vec::new();
pub(super) const ALREADY_FETCHED: bool = true;
pub(super) const NOT_FETCHED: bool = false;

pub fn init_cache() {
    unsafe {
        CACHE.reserve_exact(u32::MAX as usize);
        for _ in 0..u32::MAX {
            CACHE.push(false);
        }
        println!("Tx cache initialized");
    }
}

pub fn mark_as_fetched(hash: &H256) -> bool {
    unsafe {
        let index = convert_hash_to_index(hash);
        if CACHE[index] == ALREADY_FETCHED {
            ALREADY_FETCHED
        } else {
            CACHE[index] = ALREADY_FETCHED;
            NOT_FETCHED
        }
    }
}

pub fn was_fetched(hash: &H256) -> bool {
    unsafe {
        let index = convert_hash_to_index(hash);
        CACHE[index] == ALREADY_FETCHED
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
