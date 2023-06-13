## Info

This is the lowest layer of communication between our node and other nodes(peers). 
The connection is established in `session.rs`, via `tokio::net::TcpStream`. 
Establishing connection mens that we are communicating with node which supports [devp2p protocol](https://github.com/ethereum/devp2p), and that `Hello` messages have been successfully sent&received.
The state of connection is defined in `connection.rs`.

Upon establishing `devp2p` connection, we can receive further messages by reading/writing to/from `tcp_transport.rs`
The `TcpTransport` is implementation of `Stream` (for reading) and `Sink` (for writing) traits. 

Most of _other files_ are definitions for establishing RLPX protocol and are less "interesting". 


### NOTE/TODO: 
I think this folder has files which are not _that_ related to `RLPX`, namely `tcp_transport.rs` and (maybe) `session.rs`.
Think about moving them elsewhere. 
