use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, Stream, TryStreamExt};
use open_fastrlp::Decodable;
use tracing::{info, trace};

use super::protocol::{ProtocolVersion, ProtocolVersionError};
use crate::eth::types::status_message::Status;
use crate::rlpx::{RLPXError, RLPXMsg, RLPXSessionError};
use crate::types::hash::H512;
use crate::types::message::{Message, MessageKind};

pub trait RLPXSink: Sink<RLPXMsg, Error = RLPXError> {}
impl<T> RLPXSink for T where T: Sink<RLPXMsg, Error = RLPXError> {}

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
    pub async fn read_messages<T>(&self, mut tcp_stream: T) -> Result<(), RLPXSessionError>
    where
        T: Stream<Item = Result<RLPXMsg, RLPXError>> + Unpin,
    {
        loop {
            let msg = tcp_stream
                .try_next()
                .await?
                .ok_or(RLPXSessionError::NoMessage)?;

            if let RLPXMsg::Message(m) = msg {
                handle_messages(m)?;
            } else {
                return Err(RLPXSessionError::ExpectedRLPXMessage);
            }
        }
    }

    pub fn write_message(&self) {}
}

fn handle_messages(bytes: BytesMut) -> Result<(), RLPXSessionError> {
    let mut msg = Message::new(bytes);
    let msg_id = msg.decode_id()?;
    let msg_kind = msg
        .decode_kind()?
        .as_ref()
        .ok_or(RLPXSessionError::UnknownError)?;

    match msg_kind {
        MessageKind::ETH => {
            return handle_eth_message(msg_id, msg.data);
        }
        MessageKind::P2P(p2p_msg) => trace!("Got P2P msg: {:?}", p2p_msg),
    };

    Ok(())
}

fn handle_eth_message(msg_id: u8, bytes: BytesMut) -> Result<(), RLPXSessionError> {
    if msg_id != 16 {
        info!("Got ETH message with ID: {:?}", msg_id);
        return Ok(());
    }

    // 1. snappy decompress

    let decompressed_len =
        snap::raw::decompress_len(&bytes[1..]).map_err(|_| RLPXSessionError::UnknownError)?;
    let mut rlp_msg_bytes = BytesMut::zeroed(decompressed_len);
    let mut decoder = snap::raw::Decoder::new();

    decoder
        .decompress(&bytes[..], &mut rlp_msg_bytes[..])
        .map_err(|err| {
            tracing::debug!(
                ?err,
                msg=%hex::encode(&bytes[1..]),
                "error decompressing p2p message"
            );
            RLPXSessionError::UnknownError
        })?;
    // 2. parse

    let msg = Status::decode(&mut &rlp_msg_bytes[..])?;

    // 3. log
    info!(?msg, "Got status message");

    Ok(())
}
