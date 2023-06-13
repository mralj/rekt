use std::task::{ready, Poll};

use bytes::BytesMut;
use futures::{Sink, Stream};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::trace;

use super::{Connection, RLPXMsg, RLPXSessionError};

#[pin_project::pin_project]
#[derive(Debug)]
pub struct TcpTransport {
    #[pin]
    inner: Framed<TcpStream, Connection>,
}

impl TcpTransport {
    pub fn new(transport: Framed<TcpStream, Connection>) -> Self {
        Self { inner: transport }
    }
}

impl Stream for TcpTransport {
    type Item = Result<BytesMut, RLPXSessionError>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match ready!(self.project().inner.poll_next(cx)) {
            Some(Ok(RLPXMsg::Message(msg))) => Poll::Ready(Some(Ok(msg))),
            Some(_) => {
                trace!("Received non-message RLPX message");
                Poll::Ready(Some(Err(RLPXSessionError::ExpectedRLPXMessage)))
            }
            None => Poll::Ready(None),
        }
    }
}

macro_rules! ready_map_err {
    ($e:expr) => {
        match ready!($e) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(RLPXSessionError::RlpxError(e))),
        }
    };
}

impl Sink<BytesMut> for TcpTransport {
    type Error = RLPXSessionError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        ready_map_err!(self.project().inner.poll_ready(cx))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        self.project()
            .inner
            .start_send(RLPXMsg::Message(item))
            .map_err(RLPXSessionError::RlpxError)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        ready_map_err!(self.project().inner.poll_flush(cx))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        ready_map_err!(self.project().inner.poll_close(cx))
    }
}
