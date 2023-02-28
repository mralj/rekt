use derive_more::{AsRef, Deref};
use fixed_hash::construct_fixed_hash;
use open_fastrlp::{RlpDecodableWrapper, RlpEncodableWrapper, RlpMaxEncodedLen};

construct_fixed_hash! {
    #[derive(AsRef, Deref, RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen)]
    pub struct H512(64);
}

construct_fixed_hash! {
    #[derive(AsRef, Deref, RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen)]
    pub struct H256(32);
}

construct_fixed_hash! {
    pub struct H128(16);
}
