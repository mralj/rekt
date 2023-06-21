use std::collections::VecDeque;
use std::task::{ready, Poll};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use open_fastrlp::{DecodeError, Encodable};


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
    snappy_decoder: snap::raw::Decoder,
    snappy_encoder: snap::raw::Encoder,
}

/*
* These are the facts about the "system" we are building:
* Only P2P messages we care for are:
* 1. Ping
* 2. Disconnect
*
* The reason disconnect is important is pretty obvious (we need to remove peer from our
* server/node)
*
* The reason why we care about PING is to keep alive connection with the other peer(s).
* ATM only official solution for BSC is their fork of GETH. The both TCP and (independently) GETH
* have timeout on connection. If we don't send any messages for some time, the connection will be
* dropped. The way GETH "takes care" of this is system of Ping/Pong messages (which are also part
* of official `devp2p` spec).
* But here is thing:
* We don't really have to send our Ping messages (no one is "forcing" us to)
* We don't really have to respond to Ping messages with our Pongs (we won't be dropped if we
* don't), but we must make sure that messages are exchanged between our node and peer, or we'll be
* dropped (due to GETH/TCP timeout).
*
* I've decided to:
* 1. Not send our Ping messages
* 2. Respond to Ping messages with our Pongs, but only if there is need for this
*
* To elaborate on second point. As already mentioned, we really don't have to send reply to Ping,
* so what I decided to do is to send Ping only if there are no other messages already queued to be
* sent. This way we make sure that connection is kept alive, but we don't send necessary messages
* if we don't have to.
*
*
* The `P2PWire` takes care of  messages in way described above (reacts to Pings/Disconnects, and
* filters out all other P2P messages). If the message is not P2P and is valid ETH message it is
* passed "forward".
*
* */

impl P2PWire {
    pub fn new(rlpx_wire: TcpTransport) -> Self {
        Self {
            inner: rlpx_wire,
            writer_queue: VecDeque::with_capacity(MAX_WRITER_QUEUE_SIZE + 1),
            snappy_decoder: snap::raw::Decoder::default(),
            snappy_encoder: snap::raw::Encoder::new(),
        }
    }

    fn handle_p2p_msg(
        &mut self,
        msg: &P2PMessage,
        cx: &mut std::task::Context<'_>,
    ) -> Result<(), RLPXSessionError> {
        match msg {
            P2PMessage::Ping => {
                let no_need_to_send_ping_if_there_are_messages_queued =
                    !self.writer_queue.is_empty();
                if no_need_to_send_ping_if_there_are_messages_queued {
                    return Ok(());
                }

                let mut buf = BytesMut::new();
                P2PMessage::Pong.encode(&mut buf);

                self.writer_queue.push_back(buf);

                // Flushes (writes) sink (maybe writes our Pong message)
                // To explain "maybe writes" our Pong message:
                // If inner sink is busy our Ping message won't be written at this time
                // But this is ok, it just means that we were just "now" in process of sending
                // "some" message to a peer. And as already mentioned we Really don't have to reply
                // to Pings with Pong, we just need to keep connection alive by sending proper
                // p2p/eth msgs.
                // This is why it is ok to use poll_flush_unpin here and not poll again, because
                // "we don't care"
                let _ = self.poll_flush_unpin(cx);

                Ok(())
            }
            P2PMessage::Disconnect(r) => Err(RLPXSessionError::DisconnectRequested(*r)),
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

            if !message_is_of_interest(msg.id.unwrap()) {
                continue;
            }

            msg.snappy_decompress(&mut this.snappy_decoder)?;

            if let Err(e) = msg.decode_kind() {
                return Poll::Ready(Some(Err(RLPXSessionError::MessageDecodeError(e))));
            }

            match msg.kind.as_ref().unwrap() {
                MessageKind::ETH => return Poll::Ready(Some(Ok(msg))),
                MessageKind::P2P(m) => {
                    if let Err(e) = this.handle_p2p_msg(m, cx) {
                        return Poll::Ready(Some(Err(e)));
                    }
                }
            }
        }
        Poll::Pending
    }
}

fn message_is_of_interest(msg_id: u8) -> bool {
    match msg_id {
        1 => true,  // P2P/Disconnect
        2 => true,  // P2P/Ping
        16 => true, // ETH/Status
        27 => true, // ETH/UpgradeStatus
        18 => true, // ETH/Transactions
        26 => true, // ETH/PooledTransactions
        24 => true, // ETH/NewPoolTransactionHashes
        19 => true, // ETH/GetBlockHeaders
        21 => true, // ETH/GetBlockBodies
        _ => false,
    }
}

impl Sink<Message> for P2PWire {
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

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
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
        let mut compressed = BytesMut::zeroed(1 + snap::raw::max_compress_len(item.data.len()));
        let compressed_size = self
            .snappy_encoder
            .compress(&item.data, &mut compressed[1..])
            .map_err(|_err| {
                RLPXSessionError::MessageDecodeError(DecodeError::Custom(
                    "Could not snappy compress",
                ))
            })?;

        compressed[0] = item.id.unwrap();
        compressed.truncate(compressed_size + 1);

        self.writer_queue.push_back(compressed);
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
