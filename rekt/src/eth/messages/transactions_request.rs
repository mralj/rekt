use crate::types::hash::H256;

use bytes::{Bytes, BytesMut};
use open_fastrlp::{Encodable, RlpEncodable};

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable)]
pub struct TransactionsRequest {
    id: u64,
    hashes: Vec<H256>,
}

impl TransactionsRequest {
    pub fn new(hashes: Vec<H256>) -> Self {
        Self { id: 0, hashes }
    }

    pub fn rlp_encode(&self) -> Bytes {
        let mut rlp = BytesMut::new();
        self.encode(&mut rlp);
        rlp.freeze()
    }
}
