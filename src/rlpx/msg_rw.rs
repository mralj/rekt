use bytes::BytesMut;
use cipher::StreamCipher;
use fixed_hash::byteorder::{BigEndian, ReadBytesExt};

use crate::types::hash::H128;

use super::{
    errors::RLPXError,
    mac::{HeaderBytes, HEADER_SIZE, MAC_SIZE},
    utils::split_at_mut,
    Connection,
};

// form the docs:
// frame-size = length of frame-data, encoded as a 24bit big-endian integer
const FRAME_SIZE_DESCRIPTOR_SIZE: usize = 3; // 24 bits

impl Connection {
    pub(super) fn read_header(&mut self, data: &mut BytesMut) -> Result<usize, RLPXError> {
        //TODO: After you are sure everything is working, remove MAC and AES validation
        // in practice we don't need to validate MACand we'll be saving couple of microseconds
        // NOTE, code below calling ingres_aes is not any kind of validation, it's just decryption
        //_______
        let (header_bytes, mac_bytes) = split_at_mut(data, HEADER_SIZE)?;
        let mut header: HeaderBytes = header_bytes.try_into()?;
        let mac = H128::from_slice(&mac_bytes[..MAC_SIZE]);

        self.ingress_mac.as_mut().unwrap().update_header(&header);

        let check_mac = self.ingress_mac.as_mut().unwrap().digest();
        if check_mac != mac {
            return Err(RLPXError::TagCheckHeaderFailed);
        }
        //_______

        self.ingress_aes
            .as_mut()
            .unwrap()
            .apply_keystream(&mut header);

        if header.as_slice().len() < FRAME_SIZE_DESCRIPTOR_SIZE {
            return Err(RLPXError::InvalidHeader);
        }

        let body_size = usize::try_from(
            header
                .as_slice()
                .read_uint::<BigEndian>(FRAME_SIZE_DESCRIPTOR_SIZE)?,
        )?;

        self.body_size = Some(body_size);

        Ok(self.body_size.unwrap())
    }

    pub fn read_body<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a mut [u8], RLPXError> {
        let (body, mac_bytes) = split_at_mut(data, data.len() - MAC_SIZE)?;
        //TODO: after you are sure everything is working, remove MAC validation
        //__________
        let mac = H128::from_slice(mac_bytes);
        self.ingress_mac.as_mut().unwrap().update_body(body);
        let check_mac = self.ingress_mac.as_mut().unwrap().digest();
        if check_mac != mac {
            return Err(RLPXError::TagCheckBodyFailed);
        }
        //__________

        let size = self.body_size.unwrap();
        self.body_size = None;
        self.ingress_aes.as_mut().unwrap().apply_keystream(body);
        Ok(split_at_mut(body, size)?.0)
    }
}
