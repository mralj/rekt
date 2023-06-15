use std::collections::VecDeque;
use std::task::{ready, Poll};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use open_fastrlp::Encodable;
use tracing::info;

use crate::p2p::P2PMessage;
use crate::rlpx::{RLPXSessionError, TcpTransport};
use crate::types::message::{Message, MessageKind};

const MAX_WRITER_QUEUE_SIZE: usize = 10; // how many messages are we queuing for write

#[pin_project::pin_project]
#[derive(Debug)]
pub struct P2PWire {
    #[pin]
    inner: TcpTransport,
    writer_queue: VecDeque<BytesMut>,
}

impl P2PWire {
    pub fn new(rlpx_wire: TcpTransport) -> Self {
        Self {
            inner: rlpx_wire,
            writer_queue: VecDeque::with_capacity(MAX_WRITER_QUEUE_SIZE + 1),
        }
    }

    fn handle_p2p_msg(&mut self, msg: &P2PMessage) -> Result<(), RLPXSessionError> {
        match msg {
            P2PMessage::Ping => {
                let no_need_to_send_ping_if_there_are_messages_queued =
                    !self.writer_queue.is_empty();
                if no_need_to_send_ping_if_there_are_messages_queued {
                    return Ok(());
                }

                info!("Received Ping message, replying with Pong");
                let mut buf = BytesMut::new();
                P2PMessage::Pong.encode(&mut buf);
                info!("Sending Pong message {:?}", hex::encode(&buf));

                self.writer_queue.push_back(buf);
                Ok(())
            }
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

                    if !this.writer_queue.is_empty() {
                        this.poll_ready_unpin(cx);
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
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.as_mut();

        // this here checks if inner sink can send data
        // if it can, we "force send" the data (by calling flush)
        // if the inner sink is ready, then that implies that this sink is ready as well (as it can
        // for sure send data to "inner")
        match this.inner.poll_ready_unpin(cx) {
            Poll::Pending => {}
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(())) => {
                if this.poll_flush(cx).is_ready() {
                    return Poll::Ready(Ok(()));
                }
            }
        }

        // on the other hand if inner sink is not ready to accept new values, we have to check if we
        // are hitting the limit of the queue, if not, we just queue message and return that we are
        // ready
        // else we are in pending state
        if self.writer_queue.len() < MAX_WRITER_QUEUE_SIZE {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        // since the interacting with sink should work as follows:
        // 1. call poll_ready, if it returns Ready(ok), call start_send,
        // but if it returns anything other than that, start_send should not be called
        // in poll_ready we make sure to return Ready(Ok) if the queue is not full,
        // we should not be in situation where this method was called and queue is full, so smth.
        // bad happened, return err
        if self.writer_queue.len() > MAX_WRITER_QUEUE_SIZE {
            //TODO: add proper err here
            return Err(RLPXSessionError::UnknownError);
        }

        self.writer_queue.push_back(item);
        Ok(())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        // while there are messages in the queue and inner sink is able to send them
        // send the one by one
        loop {
            match ready!(this.inner.as_mut().poll_flush(cx)) {
                Err(err) => return Poll::Ready(Err(err)),
                Ok(()) => {
                    if let Some(message) = this.writer_queue.pop_front() {
                        if let Err(err) = this.inner.as_mut().start_send(message) {
                            return Poll::Ready(Err(err));
                        }
                    } else {
                        // there are no messages on queue, we are done writing
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        }
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        // when closing sink, if possible write all items to inner sink
        ready!(self.as_mut().poll_flush(cx))?;

        Poll::Ready(Ok(()))
    }
}
