#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    pub list: bool,
    pub payload_length: usize,
}

impl From<HeaderLen> for Header {
    fn from(value: HeaderLen) -> Self {
        Self {
            list: value.list,
            payload_length: value.payload_length,
        }
    }
}

pub struct HeaderLen {
    pub payload_length: usize,
    pub advance: usize,
    pub total_length: usize,
    pub(crate) list: bool,
}

impl HeaderLen {
    pub fn new(payload_length: usize, advance: usize, list: bool) -> Self {
        Self {
            payload_length,
            advance,
            list,
            total_length: payload_length + advance,
        }
    }
}

pub const EMPTY_STRING_CODE: u8 = 0x80;
pub const EMPTY_LIST_CODE: u8 = 0xC0;
