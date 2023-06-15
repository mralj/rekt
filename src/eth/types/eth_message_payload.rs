use bytes::BytesMut;
use open_fastrlp::Encodable;

use crate::types::message::Message;

pub struct EthMessagePayload {
    pub id: u8,
    pub data: BytesMut,
}

impl EthMessagePayload {
    pub fn new(id: u8, msg: impl Encodable) -> Self {
        let mut rlp_msg = BytesMut::new();
        msg.encode(&mut rlp_msg);
        Self { id, data: rlp_msg }
    }
}
impl TryFrom<Message> for EthMessagePayload {
    type Error = &'static str;
    fn try_from(msg: Message) -> Result<Self, &'static str> {
        let decompressed_len = snap::raw::decompress_len(&msg.data)
            .map_err(|_| "Could not read length for snappy decompress")?;
        let mut rlp_msg_bytes = BytesMut::zeroed(decompressed_len);
        let mut decoder = snap::raw::Decoder::new();

        decoder
            .decompress(&msg.data, &mut rlp_msg_bytes)
            .map_err(|err| {
                tracing::debug!(
                    ?err,
                    msg=%hex::encode(&msg.data),
                    "error decompressing p2p message"
                );
                "Could not decompress Status message"
            })?;

        Ok(EthMessagePayload {
            id: msg.id.unwrap(),
            data: rlp_msg_bytes,
        })
    }
}
