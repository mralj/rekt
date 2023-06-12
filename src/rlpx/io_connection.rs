use std::{
    io,
    task::{ready, Poll},
};

use bytes::BytesMut;
use futures::{Sink, Stream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
use tracing::trace;

use super::{Connection, RLPXError, RLPXMsg};

#[pin_project::pin_project]
pub struct ConnectionIo<Io> {
    #[pin]
    transport: Framed<Io, Connection>,
}

impl<T> ConnectionIo<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(transport: Framed<T, Connection>) -> Self {
        Self { transport }
    }
}

impl<T> Stream for ConnectionIo<T>
where
    T: AsyncRead + Unpin,
{
    type Item = Result<BytesMut, RLPXError>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match ready!(self.project().transport.poll_next(cx)) {
            Some(Ok(RLPXMsg::Message(msg))) => Poll::Ready(Some(Ok(msg))),
            Some(_) => {
                trace!("Received non-message RLPX message");
                Poll::Ready(Some(Err(RLPXError::InvalidMsgData)))
            }
            None => Poll::Ready(None),
        }
    }
}

impl<T> Sink<BytesMut> for ConnectionIo<T>
where
    T: AsyncWrite + Unpin,
{
    type Error = RLPXError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().transport.poll_ready(cx)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        let msg = RLPXMsg::Message(item);
        self.project().transport.start_send(msg)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().transport.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().transport.poll_close(cx)
    }
}
