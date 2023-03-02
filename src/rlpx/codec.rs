use bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use tracing::trace;

use super::connection::RLPXConnectionState;
///
/// Represents message received over RLPX connection from peer
#[derive(Debug, PartialEq, Eq)]
pub enum RLPXInMsg {
    Auth,
    Ack,
    Message(BytesMut),
}

/// Represents message to be sent over RLPX connection to peer
pub enum RLPXOutMsg {
    Auth,
    Ack,
    Message(Bytes),
}

impl Decoder for super::Connection {
    type Item = RLPXInMsg;
    type Error = super::errors::RLPXError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.state {
            RLPXConnectionState::Auth => {
                trace!("Received auth, this is unexpected");
                Ok(None)
            }
            RLPXConnectionState::Ack => {
                self.read_ack(src)?;
                self.state = RLPXConnectionState::Header;
                Ok(Some(RLPXInMsg::Ack))
            }
            _ => {
                trace!("Received message");
                Ok(None)
            }
        }
    }
}

impl Encoder<RLPXOutMsg> for super::Connection {
    type Error = super::errors::RLPXError;

    fn encode(&mut self, item: RLPXOutMsg, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            RLPXOutMsg::Auth => {
                self.write_auth(dst);
                self.state = RLPXConnectionState::Ack;
                Ok(())
            }
            RLPXOutMsg::Ack => {
                trace!("Got request to write ack, this is unexpected");
                Ok(())
            }
            RLPXOutMsg::Message(_) => {
                trace!("Got request to encode msg, this is unexpected");
                Ok(())
            }
        }
    }
}
