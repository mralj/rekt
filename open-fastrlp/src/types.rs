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
    pub(crate) list: bool,
    pub(crate) payload_length: usize,
    pub(crate) advance: usize,
}

pub const EMPTY_STRING_CODE: u8 = 0x80;
pub const EMPTY_LIST_CODE: u8 = 0xC0;
