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
        //TODO: After you are sure everything is working, remove MAC and AES validation
        // in practice we don't need to validate MAC and AES and we'll be saving couple of microseconds
        let (header_bytes, mac_bytes) = split_at_mut(data, HEADER_SIZE)?;
        let header: HeaderBytes = header_bytes.try_into()?;
        let mac = H128::from_slice(&mac_bytes[..HEADER_SIZE]);

        self.ingress_mac.as_mut().unwrap().update_header(&header);

        let check_mac = self.ingress_mac.as_mut().unwrap().digest();
        if check_mac != mac {
            return Err(RLPXError::TagCheckHeaderFailed);
        }

        //NOTE: this is not mac validation this is msg decryption
        self.ingress_aes
            .as_mut()
            .unwrap()
            .apply_keystream(header_bytes);

        if header.as_slice().len() < 3 {
            return Err(RLPXError::InvalidHeader);
        }

        let body_size = usize::try_from(header.as_slice().read_uint::<BigEndian>(3)?)?;

        self.body_size = Some(body_size);

        Ok(self.body_size.unwrap())
    }

    pub fn read_body<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a mut [u8], RLPXError> {
        let (body, mac_bytes) = split_at_mut(data, data.len() - 16)?;
        //TODO: after you are sure everything is working, remove MAC validation
        let mac = H128::from_slice(mac_bytes);
        self.ingress_mac.as_mut().unwrap().update_body(body);
        let check_mac = self.ingress_mac.as_mut().unwrap().digest();
        if check_mac != mac {
            return Err(RLPXError::TagCheckBodyFailed.into());
        }

        let size = self.body_size.unwrap();
        self.body_size = None;
        //NOTE: this is not mac validation this is msg decryption
        self.ingress_aes.as_mut().unwrap().apply_keystream(body);
        Ok(split_at_mut(body, size)?.0)
    }
}
