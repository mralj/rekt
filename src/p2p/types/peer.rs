use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, Stream, TryStreamExt};
use tracing::{info, trace};

use super::protocol::{ProtocolVersion, ProtocolVersionError};
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
        MessageKind::ETH => info!("Got ETH message with ID: {:?}", msg_id),
        MessageKind::P2P(p2p_msg) => trace!("Got P2P msg: {:?}", p2p_msg),
    };

    Ok(())
}
