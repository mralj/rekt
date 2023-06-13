use std::task::{ready, Poll};

use bytes::BytesMut;
use futures::{Sink, Stream};

use crate::rlpx::{RLPXSessionError, TcpTransport};
use crate::types::message::Message;

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
}

impl Stream for P2PWire {
    type Item = Result<Message, RLPXSessionError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().inner.poll_next(cx)) {
            None => Poll::Ready(None),
            Some(Ok(bytes)) => {
                let mut msg = Message::new(bytes);
                let maybe_msg_id = msg.decode_id();
                if let Err(e) = maybe_msg_id {
                    return Poll::Ready(Some(Err(RLPXSessionError::MessageDecodeError(e))));
                }

                match msg.decode_kind() {
                    Ok(_) => Poll::Ready(Some(Ok(msg))),
                    Err(e) => Poll::Ready(Some(Err(RLPXSessionError::MessageDecodeError(e)))),
                }
            }
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
        }
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
