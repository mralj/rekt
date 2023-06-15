use std::task::{ready, Poll};

use bytes::BytesMut;
use futures::{Sink, Stream};

use crate::p2p::types::p2p_wire::P2PWire;
use crate::rlpx::RLPXSessionError;

use super::eth_message_payload::EthMessagePayload;

#[pin_project::pin_project]
#[derive(Debug)]
pub struct ETHWire {
    #[pin]
    inner: P2PWire,
}

impl From<P2PWire> for ETHWire {
    fn from(p2p_wire: P2PWire) -> Self {
        Self { inner: p2p_wire }
    }
}

impl Stream for ETHWire {
    type Item = EthMessagePayload;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().inner.poll_next(cx)) {
            None => Poll::Ready(None),
            // TODO: not sure is this the best way of handling error
            // None will indicate that we should terminate connection, maybe we should if there
            // was err? this is how geth does it
            Some(Err(_)) => Poll::Ready(None),
            Some(Ok(msg)) => match EthMessagePayload::try_from(msg) {
                Ok(m) => Poll::Ready(Some(m)),
                // TODO: not sure is this the best way of handling error
                // None will indicate that we should terminate connection, maybe we should if there
                // was err? this is how geth does it
                Err(_) => Poll::Ready(None),
            },
        }
    }
}

impl Sink<EthMessagePayload> for ETHWire {
    type Error = RLPXSessionError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: EthMessagePayload,
    ) -> Result<(), Self::Error> {
        let mut encoder = snap::raw::Encoder::new();
        let mut compressed = BytesMut::zeroed(1 + snap::raw::max_compress_len(item.data.len()));
        let compressed_size =
            encoder
                .compress(&item.data, &mut compressed[1..])
                .map_err(|err| {
                    tracing::debug!(
                        ?err,
                        msg=%hex::encode(&item.data[1..]),
                        "error compressing disconnect"
                    );
                    RLPXSessionError::UnknownError
                })?;

        compressed[0] = item.id;
        compressed.truncate(compressed_size + 1);

        self.project().inner.start_send(compressed)
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
