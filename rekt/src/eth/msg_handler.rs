use open_fastrlp::Decodable;

use crate::types::hash::H256;
use crate::types::message::Message;

use super::types::errors::ETHError;
use super::types::transaction::decode_txs;

pub fn handle_eth_message(msg: Message) -> Result<(), ETHError> {
    match msg.id {
        Some(18) => handle_txs(msg),
        Some(24) => handle_tx_hashes(msg),
        _ => Ok(()),
    }
}

fn handle_tx_hashes(msg: Message) -> Result<(), ETHError> {
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

    let _hashes: Vec<H256> = Vec::decode(&mut &msg.data[..])?;

    Ok(())
}

fn handle_txs(msg: Message) -> Result<(), ETHError> {
    decode_txs(&mut &msg.data[..]);
    Ok(())
}
