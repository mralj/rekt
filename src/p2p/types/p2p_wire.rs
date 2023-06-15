use std::task::Poll;

use bytes::BytesMut;
use futures::{Sink, Stream, StreamExt};
use tracing::info;

use crate::p2p::P2PMessage;
use crate::rlpx::{RLPXSessionError, TcpTransport};
use crate::types::message::{Message, MessageKind};

#[pin_project::pin_project]
#[derive(Debug)]
pub struct P2PWire {
    #[pin]
    inner: TcpTransport,
}

impl P2PWire {
    pub fn new(rlpx_wire: TcpTransport) -> Self {
        Self { inner: rlpx_wire }
    }

    fn handle_p2p_msg(&self, msg: &P2PMessage) -> Result<(), RLPXSessionError> {
        match msg {
            P2PMessage::Ping => {
                info!("Received Ping message, replying with Pong");
                Ok(())
            } // TODO: we have to reply pong, handle this properly
            P2PMessage::Disconnect(_) => Err(RLPXSessionError::UnknownError), // TODO: this should
            // be proper err
            P2PMessage::Hello(_) => Err(RLPXSessionError::UnknownError), // TODO: proper err here
            // NOTE: this is no-op for us, technically we should never
            // receive pong message as we don't send Pings, but we'll just ignore it
            P2PMessage::Pong => Ok(()),
        }
    }
}

impl Stream for P2PWire {
    type Item = Result<Message, RLPXSessionError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        while let Poll::Ready(bytes_r) = this.inner.poll_next_unpin(cx) {
            let bytes = match bytes_r {
                None => return Poll::Ready(None),
                Some(Err(e)) => return Poll::Ready(Some(Err(e))),
                Some(Ok(bytes)) => bytes,
            };
            let mut msg = Message::new(bytes);
            if let Err(e) = msg.decode_id() {
                return Poll::Ready(Some(Err(RLPXSessionError::MessageDecodeError(e))));
            }

            if let Err(e) = msg.decode_kind() {
                return Poll::Ready(Some(Err(RLPXSessionError::MessageDecodeError(e))));
            }

            match msg.kind.as_ref().unwrap() {
                MessageKind::ETH => return Poll::Ready(Some(Ok(msg))),
                MessageKind::P2P(m) => {
                    if let Err(e) = this.handle_p2p_msg(m) {
                        return Poll::Ready(Some(Err(e)));
                    }
                }
            }
        }
        Poll::Pending
    }
}

impl Sink<BytesMut> for P2PWire {
    type Error = RLPXSessionError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        // TODO: depending on how I decide to implement "things"
        // we'll maybe add here snappy compression
        self.project().inner.start_send(item)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx)
    }
}
