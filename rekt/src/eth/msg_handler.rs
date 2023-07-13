use open_fastrlp::Decodable;

use crate::types::hash::H256;
use crate::types::message::Message;

use super::types::errors::ETHError;
use super::types::transaction::{decode_txs, TransactionRequest, TX_HASHES};

pub async fn handle_eth_message(msg: Message) -> Result<Option<Message>, ETHError> {
    match msg.id {
        Some(18) => handle_txs(msg, true).await,
        Some(24) => handle_tx_hashes(msg).await,
        Some(26) => handle_txs(msg, false).await,
        _ => Ok(None),
    }
}

async fn handle_tx_hashes(msg: Message) -> Result<Option<Message>, ETHError> {
    //NOTE: we can optimize this here is how this works "under the hood":
    //
    // fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
    //     let h = Header::decode(buf)?;
    //     if !h.list {
    //         return Err(DecodeError::UnexpectedString)
    //     }

    //     let payload_view = &mut &buf[..h.payload_length];

    //     let mut to = alloc::vec::Vec::new();
    //     while !payload_view.is_empty() {
    //         to.push(E::decode(payload_view)?);
    //     }

    //     buf.advance(h.payload_length);

    //     Ok(to)
    // }
    // the main issue with this is that we can know in advance how many elements we can allocate
    // for Vec as RLP holds the size of the data, also we could use smth. like smallvec instead of
    // vector to allocate on stack
    // this usually takes couple hundred of `ns` to decode with occasional spikes to 2 <`us`

    let anno_hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;
    let tx_hashes = TX_HASHES.read().await;
    let hashes = anno_hashes
        .iter()
        .filter(|h| !tx_hashes.contains(h))
        .collect();

    Ok(Some(Message {
        id: Some(25),
        kind: Some(crate::types::message::MessageKind::ETH),
        data: TransactionRequest::new(hashes).rlp_encode(),
    }))
}

async fn handle_txs(msg: Message, is_direct: bool) -> Result<Option<Message>, ETHError> {
    decode_txs(&mut &msg.data[..], is_direct).await;
    Ok(None)
}
