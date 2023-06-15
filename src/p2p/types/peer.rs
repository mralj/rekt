use std::fmt::{Display, Formatter};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};

use open_fastrlp::Decodable;
use tracing::{error, info, trace};

use super::protocol::ProtocolVersion;
use crate::eth::types::eth_message_payload::EthMessagePayload;
use crate::eth::types::status_message::{Status, UpgradeStatus};
use crate::rlpx::RLPXSessionError;
use crate::types::hash::H512;
use crate::types::node_record::NodeRecord;

pub trait RLPXStream: Stream<Item = EthMessagePayload> + Unpin {}
impl<T> RLPXStream for T where T: Unpin + Stream<Item = EthMessagePayload> {}

pub trait RLPXSink: Sink<EthMessagePayload, Error = RLPXSessionError> + Unpin {}
impl<T> RLPXSink for T where T: Unpin + Sink<EthMessagePayload, Error = RLPXSessionError> {}

#[derive(Debug)]
pub struct P2PPeer<R: RLPXStream, W: RLPXSink> {
    node_record: NodeRecord,
    id: H512,
    protocol_version: ProtocolVersion,
    writer: W,
    reader: R,
}

impl<R: RLPXStream, W: RLPXSink> P2PPeer<R, W> {
    pub fn new(enode: NodeRecord, id: H512, protocol: usize, r: R, w: W) -> Self {
        Self {
            id,
            reader: r,
            writer: w,
            node_record: enode,
            protocol_version: ProtocolVersion::from(protocol),
        }
    }
}

impl<R: RLPXStream, W: RLPXSink> Display for P2PPeer<R, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "enode: {}, id: {}, protocol v.: {}",
            self.node_record.str, self.id, self.protocol_version
        )
    }
}

impl<R: RLPXStream, W: RLPXSink> P2PPeer<R, W> {
    pub async fn run(&mut self) -> Result<(), RLPXSessionError> {
        self.handshake().await?;
        loop {
            let msg = self
                .reader
                .next()
                .await
                // by stream definition when Poll::Ready(None) is returned this means that
                // stream is done and should not be polled again, or bad things will happen
                .ok_or(RLPXSessionError::NoMessage)?; //
            self.handle_eth_message(msg).await?;
        }
    }

    pub async fn send_our_status_msg(&mut self) -> Result<(), RLPXSessionError> {
        self.writer
            .send(EthMessagePayload::new(
                16,
                Status::make_our_status_msg(&self.protocol_version),
            ))
            .await?;

        self.writer
            .send(EthMessagePayload::new(
                0x10 + 0x0b,
                UpgradeStatus::default(),
            ))
            .await
    }

    async fn handle_eth_message(&mut self, msg: EthMessagePayload) -> Result<(), RLPXSessionError> {
        let msg_id_is_bsc_upgrade_status_msg = msg.id == 27;
        if !msg_id_is_bsc_upgrade_status_msg {
            //   info!("Got ETH message with ID: {:?}", msg.id);
        } else {
            info!("Got upgrade status msg");
        }

        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), RLPXSessionError> {
        let msg = self
            .reader
            .next()
            .await
            .ok_or(RLPXSessionError::NoMessage)?;

        if msg.id != 16 {
            error!("Expected status message, got {:?}", msg.id);
            return Err(RLPXSessionError::UnknownError);
        }

        let status_msg = Status::decode(&mut &msg.data[..])?;

        if Status::validate(&status_msg, &self.protocol_version).is_err() {
            return Err(RLPXSessionError::UnknownError);
        } else {
            info!("Validated status MSG OK");
        }

        self.send_our_status_msg().await
    }
}
