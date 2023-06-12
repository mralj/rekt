use std::fmt::Display;

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
use tracing::trace;

use crate::rlpx::ecies::RLPX_AUTH_MSG_LEN_MARKER;

use super::{
    connection::RLPXConnectionState,
    errors::RLPXError,
    mac::{HEADER_SIZE, MAC_SIZE},
};

/// NOTE: this module handles RLPX framing, using Tokio codec
/// The official docs are pretty good explaining how to use this: https://docs.rs/tokio-util/0.7.7/tokio_util/codec/index.html
/// Especially helpful were their implementations of length delimited codec:
/// https://docs.rs/tokio-util/0.7.7/src/tokio_util/codec/length_delimited.rs.html#1-1043
/// And lines codec
/// https://docs.rs/tokio-util/0.7.7/src/tokio_util/codec/lines_codec.rs.html#12-28

/// Represents message received over RLPX connection from peer
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RLPXMsg {
    Auth,
    Ack,
    Message(BytesMut),
}

impl Display for RLPXMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RLPXMsg::Auth => write!(f, "Auth"),
            RLPXMsg::Ack => write!(f, "Ack"),
            RLPXMsg::Message(m) => write!(f, "Message: {:?}", m),
        }
    }
}

const SIGNAL_TO_TCP_STREAM_MORE_DATA_IS_NEEDED: Result<Option<RLPXMsg>, RLPXError> = Ok(None);
const RLPX_MSG_HEADER_LEN: usize = HEADER_SIZE + MAC_SIZE;

impl Decoder for super::Connection {
    type Item = RLPXMsg;
    type Error = RLPXError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self.state {
                RLPXConnectionState::Auth => {
                    trace!("Received auth, this is unexpected");
                    return Err(RLPXError::UnexpectedMessage {
                        received: RLPXMsg::Auth,
                        expected: RLPXMsg::Ack,
                    });
                }
                RLPXConnectionState::Ack => {
                    // At minimum we  need 2 bytes, because per RLPX spec
                    // The first 2 bytes of the packet carry the size of msg
                    if src.len() < RLPX_AUTH_MSG_LEN_MARKER {
                        return SIGNAL_TO_TCP_STREAM_MORE_DATA_IS_NEEDED;
                    }

                    let payload_size = u16::from_be_bytes([src[0], src[1]]) as usize;
                    let total_size = payload_size + RLPX_AUTH_MSG_LEN_MARKER;

                    if src.len() < total_size {
                        // small perf optimization, suggested in the docs
                        src.reserve(total_size - src.len());
                        return SIGNAL_TO_TCP_STREAM_MORE_DATA_IS_NEEDED;
                    }

                    // NOTE: the split_to here will pass "new" buffer to handler
                    // leaving the Decoder with buffer which contains remaining data [total_size, len>
                    // this is neat way of getting the exact frame we need
                    // whilst respecting the requirement of the Decoder trait
                    // From the docs:
                    // The decoder should use a method such as split_to or advance to modify the buffer such that the frame is removed from the buffer,
                    // but any data in the buffer after that frame should still remain in the buffer.
                    // The decoder should also return Ok(Some(the_decoded_frame)) in this case.
                    self.read_ack(&mut src.split_to(total_size))?;
                    self.state = RLPXConnectionState::Header;
                    return Ok(Some(RLPXMsg::Ack));
                }
                RLPXConnectionState::Header => {
                    if src.len() < RLPX_MSG_HEADER_LEN {
                        return SIGNAL_TO_TCP_STREAM_MORE_DATA_IS_NEEDED;
                    }

                    let _expected_msg_body_size =
                        self.read_header(&mut src.split_to(RLPX_MSG_HEADER_LEN))?;
                    self.state = RLPXConnectionState::Body;
                }
                RLPXConnectionState::Body => {
                    let len_with_padding = self.body_size_rounded_up_to_multiple_of_frame_padding();
                    if src.len() < len_with_padding {
                        return SIGNAL_TO_TCP_STREAM_MORE_DATA_IS_NEEDED;
                    }

                    let mut data = src.split_to(len_with_padding);
                    //TODO: future possible nano-opt, do not create new buffer here, use one we
                    //already have
                    let mut ret = BytesMut::new();
                    ret.extend_from_slice(self.read_body(&mut data)?);

                    self.state = RLPXConnectionState::Header;
                    return Ok(Some(RLPXMsg::Message(ret)));
                }
            }
        }
    }
}

impl Encoder<RLPXMsg> for super::Connection {
    type Error = super::errors::RLPXError;

    fn encode(&mut self, item: RLPXMsg, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            RLPXMsg::Auth => {
                self.write_auth(dst);
                self.state = RLPXConnectionState::Ack;
                Ok(())
            }
            RLPXMsg::Ack => {
                trace!("Got request to write ack, this is unexpected at this time ");
                Ok(())
            }
            RLPXMsg::Message(msg) => {
                self.write_header(dst, msg.len());
                self.write_body(dst, &msg);
                Ok(())
            }
        }
    }
}
