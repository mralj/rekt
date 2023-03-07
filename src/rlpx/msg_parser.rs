use bytes::BytesMut;
use cipher::StreamCipher;
use fixed_hash::byteorder::{BigEndian, ReadBytesExt};

use crate::types::hash::H128;

use super::{
    errors::RLPXError,
    mac::{HeaderBytes, HEADER_SIZE},
    utils::split_at_mut,
    Connection,
};

impl Connection {
    pub(super) fn read_header(&mut self, data: &mut BytesMut) -> Result<usize, RLPXError> {
        let (header_bytes, mac_bytes) = split_at_mut(data, HEADER_SIZE)?;
        //TODO: remove unwrap
        let header: HeaderBytes = header_bytes.try_into().unwrap();
        let mac = H128::from_slice(&mac_bytes[..HEADER_SIZE]);

        self.ingress_mac.as_mut().unwrap().update_header(&header);

        let check_mac = self.ingress_mac.as_mut().unwrap().digest();
        if check_mac != mac {
            return Err(RLPXError::TagCheckHeaderFailed);
        }

        self.ingress_aes
            .as_mut()
            .unwrap()
            .apply_keystream(header_bytes);

        if header.as_slice().len() < 3 {
            return Err(RLPXError::InvalidHeader);
        }

        //TODO: remove unwrap
        let body_size = usize::try_from(header.as_slice().read_uint::<BigEndian>(3)?).unwrap();

        self.body_size = Some(body_size);

        Ok(self.body_size.unwrap())
    }
}
