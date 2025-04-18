use derive_more::{AsRef, Deref};
use fixed_hash::construct_fixed_hash;
use impl_serde::impl_fixed_hash_serde;
use open_fastrlp::{RlpDecodableWrapper, RlpEncodableWrapper, RlpMaxEncodedLen};
use serde::{Deserialize, Serialize};

construct_fixed_hash! {
    #[derive(AsRef, Deref, RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen)]
    pub struct H512(64);
}
impl_fixed_hash_serde!(H512, 64);

construct_fixed_hash! {
    #[derive(AsRef, Deref, RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen, Serialize, Deserialize)]
    pub struct H256(32);
}

construct_fixed_hash! {
    #[derive(AsRef, Deref, RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen, Serialize, Deserialize)]
    pub struct H160(20);
}

construct_fixed_hash! {
    pub struct H128(16);
}

impl H512 {
    pub fn distance(&self, other: &H512) -> H512 {
        let mut distance = [0u8; 64];

        for i in 0..64 {
            distance[i] = self[i] ^ other[i];
        }

        H512::from(distance)
    }
}
