use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, TryStreamExt};
use open_fastrlp::Decodable;
use tracing::{info, trace};

use super::protocol::ProtocolVersion;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::rlpx::RLPXSessionError;
use crate::types::hash::H512;
use crate::types::message::{Message, MessageKind};
use crate::types::node_record::NodeRecord;

pub trait RLPXStream: Stream<Item = Result<Message, RLPXSessionError>> + Unpin {}
impl<T> RLPXStream for T where T: Unpin + Stream<Item = Result<Message, RLPXSessionError>> {}

pub trait RLPXSink: Sink<BytesMut, Error = RLPXSessionError> + Unpin {}
impl<T> RLPXSink for T where T: Unpin + Sink<BytesMut, Error = RLPXSessionError> {}

#[derive(Debug)]
pub struct P2PPeer<R: RLPXStream, W: RLPXSink> {
    node_record: NodeRecord,
    id: H512,
    protocol_version: ProtocolVersion,
    writer: W,
    reader: R,
}

impl<R: RLPXStream, W: RLPXSink> P2PPeer<R, W> {
    pub fn new(enode: NodeRecord, id: H512, protocol: usize, r: R, w: W) -> Self {
        Self {
            id,
            reader: r,
            writer: w,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
        }
    }
}

impl<R: RLPXStream, W: RLPXSink> Display for P2PPeer<R, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.node_record.str, self.id, self.protocol_version
        )
    }
}

impl<R: RLPXStream, W: RLPXSink> P2PPeer<R, W> {
    pub async fn read_messages(&mut self) -> Result<(), RLPXSessionError> {
        loop {
            let msg = self
                .reader
                .try_next()
                .await?
                // by stream definition when Poll::Ready(None) is returned this means that
                // stream is done and should not be polled again, or bad things will happen
                .ok_or(RLPXSessionError::NoMessage)?; //
            self.handle_messages(msg).await?;
        }
    }

    pub async fn write_message(&mut self, msg: BytesMut) -> Result<(), RLPXSessionError> {
        self.writer.send(msg).await?;
        Ok(())
    }

    pub async fn send_our_status_msg(&mut self) -> Result<(), RLPXSessionError> {
        let rlp_msg = Status::make_our_status_msg(&self.protocol_version).rlp_encode();

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

        self.write_message(compressed).await?;
        let rlp_msg = UpgradeStatus::default().rl_encode();

        trace!(
            "Sending upgrade status extension: {:?}",
            hex::encode(&rlp_msg)
        );

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
        compressed[0] = 0x10 + 0x0b;
        compressed.truncate(compressed_size + 1);
        self.write_message(compressed).await
    }

    async fn handle_messages(&mut self, msg: Message) -> Result<(), RLPXSessionError> {
        if msg.kind.is_none() {
            return Err(RLPXSessionError::UnknownError);
        }

        match msg.kind.unwrap() {
            MessageKind::ETH => {
                return self.handle_eth_message(msg.id.unwrap(), msg.data).await;
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
        if msg_id == 27 {
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
            trace!(
                "Upgrade status extension message received {:?}",
                hex::encode(&rlp_msg_bytes)
            );
        }

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

        if Status::validate(&msg, &self.protocol_version).is_err() {
            return Err(RLPXSessionError::UnknownError);
        } else {
            info!("Validated status MSG OK");
        }

        self.send_our_status_msg().await
    }

    // pub async fn handshake(&mut self) -> Result<(), RLPXSessionError> {
    //     let msg = self
    //         .reader
    //         .try_next()
    //         .await?
    //         .ok_or(RLPXSessionError::NoMessage)?;
    //     Ok(())
    // }
}
