const MAC_SIZE: usize = 32;
const SIG_SIZE: usize = 65;
const HEAD_SIZE: usize = MAC_SIZE + SIG_SIZE;

// Discovery packets are defined to be no larger than 1280 bytes.
// Packets larger than this size will be cut at the end and treated
// as invalid because their hash won't match.
const MAX_PACKET_SIZE: usize = 1280;

pub fn packet_size_is_valid(size: usize) -> bool {
    if size <= HEAD_SIZE {
        return false;
    }

    if size > MAX_PACKET_SIZE {
        return false;
    }

    true
}

pub fn decode_msg(buf: &[u8]) {
    let msg_type = buf[HEAD_SIZE..][0];

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
