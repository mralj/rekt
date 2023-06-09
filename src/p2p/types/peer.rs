use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, TryStreamExt};
use open_fastrlp::Decodable;
use tracing::{info, trace};

use super::protocol::{ProtocolVersion, ProtocolVersionError};
use crate::eth::types::status_message::Status;
use crate::rlpx::{RLPXError, RLPXMsg, RLPXSessionError};
use crate::types::hash::H512;
use crate::types::message::{Message, MessageKind};

pub trait RLPXSink: Unpin + Sink<RLPXMsg, Error = RLPXError> {}
impl<T> RLPXSink for T where T: Unpin + Sink<RLPXMsg, Error = RLPXError> {}

#[derive(Debug)]
pub struct P2PPeer<S: RLPXSink> {
    enode: String,
    id: H512,
    protocol_version: ProtocolVersion,
    writer: S,
}

impl<S: RLPXSink> P2PPeer<S> {
    pub fn new(
        enode: String,
        id: H512,
        protocol: usize,
        writer: S,
    ) -> Result<Self, ProtocolVersionError> {
        let protocol = ProtocolVersion::try_from(protocol)?;
        Ok(Self {
            enode,
            id,
            writer,
            protocol_version: protocol,
        })
    }
}

impl<S: RLPXSink> Display for P2PPeer<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.enode, self.id, self.protocol_version
        )
    }
}

impl<S: RLPXSink> P2PPeer<S> {
    pub async fn read_messages<T>(&mut self, mut tcp_stream: T) -> Result<(), RLPXSessionError>
    where
        T: Stream<Item = Result<RLPXMsg, RLPXError>> + Unpin,
    {
        loop {
            let msg = tcp_stream
                .try_next()
                .await?
                .ok_or(RLPXSessionError::NoMessage)?;

            if let RLPXMsg::Message(m) = msg {
                self.handle_messages(m).await?;
            } else {
                return Err(RLPXSessionError::ExpectedRLPXMessage);
            }
        }
    }

    pub async fn write_message(&mut self, msg: RLPXMsg) -> Result<(), RLPXSessionError> {
        self.writer.send(msg).await?;
        Ok(())
    }

    pub async fn send_our_status_msg(&mut self) -> Result<(), RLPXSessionError> {
        trace!("Sending our status message");
        let rlp_msg = Status::default().rlp_encode();
        trace!("Rlp encoded status message: {:?}", rlp_msg);

        let mut encoder = snap::raw::Encoder::new();
        let mut compressed = BytesMut::zeroed(1 + snap::raw::max_compress_len(rlp_msg.len()));
        let compressed_size = encoder
            .compress(&rlp_msg, &mut compressed[1..])
            .map_err(|err| {
                tracing::debug!(
                    ?err,
                    msg=%hex::encode(&rlp_msg[1..]),
                    "error compressing disconnect"
                );
                RLPXSessionError::UnknownError
            })?;

        // truncate the compressed buffer to the actual compressed size (plus one for the message
        // id)
        compressed[0] = 0x10;
        compressed.truncate(compressed_size + 1);

        trace!("Sending compressed status message: {:?}", compressed);

        self.write_message(RLPXMsg::Message(compressed)).await
    }

    async fn handle_messages(&mut self, bytes: BytesMut) -> Result<(), RLPXSessionError> {
        let mut msg = Message::new(bytes);
        let msg_id = msg.decode_id()?;
        let msg_kind = msg
            .decode_kind()?
            .as_ref()
            .ok_or(RLPXSessionError::UnknownError)?;

        match msg_kind {
            MessageKind::ETH => {
                return self.handle_eth_message(msg_id, msg.data).await;
            }
            MessageKind::P2P(p2p_msg) => trace!("Got P2P msg: {:?}", p2p_msg),
        };

        Ok(())
    }

    async fn handle_eth_message(
        &mut self,
        msg_id: u8,
        bytes: BytesMut,
    ) -> Result<(), RLPXSessionError> {
        if msg_id != 16 {
            info!("Got ETH message with ID: {:?}", msg_id);
            return Ok(());
        }

        // 1. snappy decompress

        let decompressed_len =
            snap::raw::decompress_len(&bytes).map_err(|_| RLPXSessionError::UnknownError)?;
        let mut rlp_msg_bytes = BytesMut::zeroed(decompressed_len);
        let mut decoder = snap::raw::Decoder::new();

        decoder
            .decompress(&bytes, &mut rlp_msg_bytes)
            .map_err(|err| {
                tracing::debug!(
                    ?err,
                    msg=%hex::encode(&bytes),
                    "error decompressing p2p message"
                );
                RLPXSessionError::UnknownError
            })?;
        // 2. parse

        let msg = Status::decode(&mut &rlp_msg_bytes[..])?;

        // 3. log
        info!(?msg, "Got status message");

        self.send_our_status_msg().await
    }
}
