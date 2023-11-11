use bytes::Bytes;
use dashmap::DashMap;
use static_init::dynamic;

#[dynamic]
pub(super) static CACHE: DashMap<Bytes, ()> =
    DashMap::with_capacity_and_shard_amount(1_000_000, 256);
