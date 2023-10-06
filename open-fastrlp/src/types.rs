#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    pub list: bool,
    pub payload_length: usize,
}

impl From<HeaderInfo> for Header {
    fn from(value: HeaderInfo) -> Self {
        Self {
            list: value.list,
            payload_length: value.payload_len,
        }
    }
}

pub struct HeaderInfo {
    pub payload_len: usize,
    pub header_len: usize,
    pub total_len: usize,
    pub list: bool,
}

impl HeaderInfo {
    pub fn new(header_len: usize, payload_len: usize, list: bool) -> Self {
        Self {
            payload_len,
            header_len,
            list,
            total_len: payload_len + header_len,
        }
    }
}

pub const EMPTY_STRING_CODE: u8 = 0x80;
pub const EMPTY_LIST_CODE: u8 = 0xC0;
