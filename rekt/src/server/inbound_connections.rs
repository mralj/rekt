use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use futures::{SinkExt, TryStreamExt};
use secp256k1::PublicKey;
use tokio::net::{TcpSocket, TcpStream};
use tokio_util::codec::{Decoder, Framed};
use tracing::error;

use crate::{
    cli::Cli,
    constants::DEFAULT_PORT,
    local_node::LocalNode,
    p2p::{
        errors::P2PError,
        peer::{is_buy_or_sell_in_progress, PeerType},
        tx_sender::PEERS_SELL,
        Peer, Protocol,
    },
    rlpx::{Connection, RLPXError, RLPXMsg, RLPXSessionError, TcpWire},
    server::peers::BLACKLIST_PEERS_BY_IP,
    types::node_record::NodeRecord,
};

use super::{
    active_peer_session::handle_hello_msg,
    peers::{PEERS, PEERS_BY_IP},
};

pub struct InboundConnections {
    our_private_key: secp256k1::SecretKey,

    is_paused: AtomicBool,
    concurrent_conn_attempts: Arc<tokio::sync::Semaphore>,

    cli: crate::cli::Cli,
}

impl InboundConnections {
    pub fn new(local_node: LocalNode, cli: Cli) -> Self {
        Self {
            our_private_key: local_node.private_key,
            is_paused: AtomicBool::new(false),
            concurrent_conn_attempts: Arc::new(tokio::sync::Semaphore::new(256)),
            cli,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn stop_listener(&self) -> bool {
        self.is_paused
            .swap(true, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn start_listener(&self) -> bool {
        self.is_paused
            .swap(false, std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn run(&self) -> Result<(), io::Error> {
        let socket = TcpSocket::new_v4()?;
        socket.set_reuseport(true)?;
        socket.set_reuseaddr(true)?;
        socket.bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_PORT,
        )))?;
        println!("TCP Server listening on {}", socket.local_addr()?);

        let our_secret_key = self.our_private_key;
        let listener = socket.listen(1024)?;
        loop {
            let semaphore = self
                .concurrent_conn_attempts
                .clone()
                .acquire_owned()
                .await
                .unwrap();

            let (stream, src) = listener.accept().await?;
            //let _ = stream.set_nodelay(true);

            if self.is_paused() {
                tokio::time::sleep(Duration::from_secs(120)).await;
                continue;
            }

            if BLACKLIST_PEERS_BY_IP.contains(&src.ip()) {
                continue;
            }

            let cli = self.cli.clone();
            tokio::spawn(async move {
                let rlpx_connection = Connection::new_in(our_secret_key);
                let transport = rlpx_connection.framed(stream);
                let _ =
                    new_connection_handler(src, transport, our_secret_key, cli, semaphore).await;
            });
        }
    }
}

async fn new_connection_handler(
    address: SocketAddr,
    mut transport: Framed<TcpStream, Connection>,
    secret_key: secp256k1::SecretKey,
    cli: Cli,
    semaphore: tokio::sync::OwnedSemaphorePermit,
) -> Result<(), RLPXSessionError> {
    let external_node_pub_key = handle_auth(&mut transport).await?;

    let node = NodeRecord::new(
        address.ip(),
        address.port(),
        address.port(),
        external_node_pub_key,
    );
    let pub_key = PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret_key);

    let (hello_msg, protocol_v) = match handle_hello_msg(&pub_key, &mut transport).await {
        Ok(mut hello_msg) => {
            let matched_protocol = Protocol::match_protocols(&mut hello_msg.protocols)
                .ok_or(RLPXSessionError::NoMatchingProtocols)?;

            (hello_msg, matched_protocol.version)
        }
        Err(e) => {
            return Err(e);
        }
    };

    drop(semaphore);
    let mut p = Peer::new(
        node.clone(),
        hello_msg.id,
        protocol_v,
        hello_msg.client_version,
        TcpWire::new(transport),
        PeerType::Inbound,
        cli,
    );

    let task_result = p.run().await;
    if is_buy_or_sell_in_progress() {
        //NOTE: don't disconnect peers immediately to avoid UB (like nil ptr)

        while is_buy_or_sell_in_progress() {
            tokio::time::sleep(Duration::from_secs(120)).await;
        }
    }
    PEERS.remove(&node.id);
    PEERS_SELL.lock().await.remove(&node.id);

    // In case we got already connected to same ip error we do not remove the IP from the set
    // of already connected ips
    // But in all other cases we must remove the IP from the set
    if !matches!(task_result, Err(P2PError::AlreadyConnectedToSameIp)) {
        PEERS_BY_IP.remove(&node.ip);
    }

    Ok(())
}

async fn handle_auth(
    transport: &mut Framed<TcpStream, Connection>,
) -> Result<PublicKey, RLPXSessionError> {
    let msg = transport
        .try_next()
        .await?
        .ok_or(RLPXError::InvalidAuthData)?;

    let pub_key = match msg {
        RLPXMsg::Auth(key) => key,
        _ => {
            error!("Got unexpected message: {:?}", msg);
            return Err(RLPXSessionError::RlpxError(RLPXError::InvalidAuthData));
        }
    };

    transport.send(crate::rlpx::codec::RLPXMsgOut::Ack).await?;
    Ok(pub_key)
}
