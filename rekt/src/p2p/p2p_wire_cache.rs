use bytes::Bytes;
use std::hash::Hasher;
use tokio::time::interval;
use tokio_stream::StreamExt;
use twox_hash::XxHash32;

pub(super) const ALREADY_CACHED: bool = true;
pub(super) const NOT_CACHED: bool = false;

pub(super) static mut CACHE_TXS: Vec<bool> = Vec::new();
pub(super) static mut CACHE_HASHES: Vec<bool> = Vec::new();

static mut TOTAL_TX: usize = 0;
static mut TOTAL_HASHES: usize = 0;

static mut TX_HIT: usize = 0;
static mut TX_MISS: usize = 0;

static mut HASH_HIT: usize = 0;
static mut HASH_MISS: usize = 0;

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
        TOTAL_HASHES += 1;
        if CACHE_HASHES[index] == true {
            HASH_HIT += 1;
            return ALREADY_CACHED;
        }
        HASH_MISS += 1;
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
        TOTAL_TX += 1;
        if CACHE_TXS[index] == true {
            TX_HIT += 1;
            return ALREADY_CACHED;
        }
        TX_MISS += 1;
        CACHE_TXS[index] = true;
    }

    NOT_CACHED
}

#[inline(always)]
fn hash(data: &Bytes) -> usize {
    let mut hasher = XxHash32::default();
    hasher.write(data);
    hasher.finish() as usize
}

pub fn logger() {
    tokio::spawn(async {
        let mut stream = tokio_stream::wrappers::IntervalStream::new(interval(
            std::time::Duration::from_secs(90),
        ));

        let started = tokio::time::Instant::now();

        while let Some(_) = stream.next().await {
            unsafe {
                println!("=== STATS ===");
                println!("Test duration: {:?} min", started.elapsed().as_secs() / 60);
                println!("TOTAL TX: {TOTAL_TX}, TOTAL HASHES {TOTAL_HASHES}");
                println!(
                    "TX CACHE HIT: {}, {}%",
                    TX_HIT,
                    f64::round(TX_HIT as f64 / TOTAL_TX as f64 * 100.0)
                );
                println!(
                    "CACHE MISS: {}, {}%",
                    TX_MISS,
                    f64::round(TX_MISS as f64 / TOTAL_TX as f64 * 100.0)
                );
                println!(
                    "HASHES CACHE HIT: {}, {}%",
                    HASH_HIT,
                    f64::round(HASH_HIT as f64 / TOTAL_HASHES as f64 * 100.0)
                );
                println!(
                    "HASHES CACHE MISS: {}, {}%",
                    HASH_MISS,
                    f64::round(HASH_MISS as f64 / TOTAL_HASHES as f64 * 100.0)
                );

                println!("=== END ===");
            }
        }
    });
}
