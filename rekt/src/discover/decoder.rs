// The following constants are defined in the "docs"
// https://github.com/ethereum/devp2p/blob/master/discv4.md
const HASH_SIZE: usize = 32;
const SIGNATURE_SIZE: usize = 65;
const TYPE_SIZE: usize = 1;
const HEADER_SIZE: usize = HASH_SIZE + SIGNATURE_SIZE;
// Discovery packets are defined to be no larger than 1280 bytes.
// Packets larger than this size will be cut at the end and treated
// as invalid because their hash won't match.
pub const MAX_PACKET_SIZE: usize = 1280;

pub fn packet_size_is_valid(size: usize) -> bool {
    if size <= HEADER_SIZE {
        return false;
    }

    if size > MAX_PACKET_SIZE {
        return false;
    }

    true
}

pub fn decode_msg_type(buf: &[u8]) {
    let hash = &buf[..HASH_SIZE];
    let signature = &buf[HASH_SIZE..HEADER_SIZE];
    let msg_type = &buf[HEADER_SIZE..][0];
    let msg_data = &buf[HEADER_SIZE + TYPE_SIZE..];

    match msg_type {
        1 => println!("Ping"),
        2 => println!("Pong"),
        3 => println!("Find NodePacket"),
        4 => println!("NeighborsPacket"),
        5 => println!("ENRRequestPacket"),
        6 => println!("ENRResponsePacket"),
        _ => println!("Unknown"),
    }
}
