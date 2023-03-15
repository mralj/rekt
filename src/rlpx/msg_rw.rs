use bytes::BytesMut;
use cipher::StreamCipher;
use fixed_hash::byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use num_integer::Integer;

use crate::types::hash::H128;

use super::{
    connection::FRAME_PADDING,
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

    pub fn write_header(&mut self, out: &mut BytesMut, size: usize) {
        let mut buf = [0u8; 8];
        BigEndian::write_uint(&mut buf, size as u64, FRAME_SIZE_DESCRIPTOR_SIZE);
        let mut header = [0u8; HEADER_SIZE];
        header[..FRAME_SIZE_DESCRIPTOR_SIZE].copy_from_slice(&buf[..FRAME_SIZE_DESCRIPTOR_SIZE]);
        header[3..6].copy_from_slice(&[194, 128, 128]); // I have 0 idea what this is

        let mut header = header;
        self.egress_aes
            .as_mut()
            .unwrap()
            .apply_keystream(&mut header);
        self.egress_mac.as_mut().unwrap().update_header(&header);
        let tag = self.egress_mac.as_mut().unwrap().digest();

        out.reserve(HEADER_SIZE + MAC_SIZE);
        out.extend_from_slice(&header);
        out.extend_from_slice(tag.as_bytes());
    }

    pub fn write_body(&mut self, out: &mut BytesMut, data: &[u8]) {
        let len_with_padding = (data.len().div_ceil(&FRAME_PADDING)) * FRAME_PADDING;
        let old_len = out.len();
        out.resize(old_len + len_with_padding, 0);

        let encrypted = &mut out[old_len..old_len + len_with_padding];
        encrypted[..data.len()].copy_from_slice(data);

        self.egress_aes.as_mut().unwrap().apply_keystream(encrypted);
        self.egress_mac.as_mut().unwrap().update_body(encrypted);
        let tag = self.egress_mac.as_mut().unwrap().digest();

        out.extend_from_slice(tag.as_bytes());
    }
}
