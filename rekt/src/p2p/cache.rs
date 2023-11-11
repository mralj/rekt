use std::hash::BuildHasherDefault;

use bytes::Bytes;
use dashmap::DashMap;
use static_init::dynamic;
use xxhash_rust::xxh3;

type DashMapWithXxHash<K, V> = DashMap<K, V, BuildHasherDefault<xxh3::Xxh3>>;
#[dynamic]
pub(super) static CACHE: DashMap<Bytes, ()> =
    DashMap::with_capacity_and_shard_amount(1_000_000, 256);
