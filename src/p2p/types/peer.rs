use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, Stream, TryStreamExt};
use tracing::{info, trace};

use super::protocol::{ProtocolVersion, ProtocolVersionError};
use crate::rlpx::{RLPXError, RLPXMsg, RLPXSessionError};
use crate::types::hash::H512;
use crate::types::message::{Message, MessageKind};

#[derive(Debug)]
pub struct P2PPeer<S>
where
    S: Sink<RLPXMsg, Error = RLPXError>,
{
    enode: String,
    id: H512,
    protocol_version: ProtocolVersion,
    writer: S,
}

impl<S> P2PPeer<S>
where
    S: Sink<RLPXMsg, Error = RLPXError>,
{
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

impl<S> Display for P2PPeer<S>
where
    S: Sink<RLPXMsg, Error = RLPXError>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.enode, self.id, self.protocol_version
        )
    }
}

impl<S> P2PPeer<S>
where
    S: Sink<RLPXMsg, Error = RLPXError>,
{
    // pub fn set_writer<T>(&self, writer: T)
    // where
    //     T: Sink<RLPXMsg, Error = RLPXError> + Unpin,
    // {
    // }

    pub async fn read_messages<T>(&self, mut tcp_stream: T) -> Result<(), RLPXSessionError>
    where
        T: Stream<Item = Result<RLPXMsg, RLPXError>> + Unpin,
    {
        loop {
            let msg = tcp_stream.try_next().await?;
            if msg.is_none() {
                return Err(RLPXSessionError::NoMessage);
            }

            if let RLPXMsg::Message(m) = msg.unwrap() {
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
    let msg_kind = msg.decode_kind()?;

    match msg_kind {
        None => Err(RLPXSessionError::UnknownError),
        Some(MessageKind::ETH) => {
            info!("Got ETH message with ID: {:?}", msg_id);
            Ok(())
        }
        Some(MessageKind::P2P(p2p_msg)) => {
            trace!("Got P2P msg: {:?}", p2p_msg);
            Ok(())
        }
    }
}
