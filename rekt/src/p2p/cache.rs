use std::hash::BuildHasherDefault;

use bytes::Bytes;
use dashmap::DashMap;
use static_init::dynamic;
use xxhash_rust::xxh3::{self, Xxh3Builder};

#[dynamic]
pub(super) static CACHE: DashMap<Bytes, ()> =
    DashMap::with_capacity_and_shard_amount(1_000_000, 256);
