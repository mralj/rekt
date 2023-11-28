use std::{collections::HashMap, usize};

use color_print::cprintln;
use futures::{future::join_all, stream::FuturesUnordered, SinkExt};
use static_init::dynamic;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::{eth::eth_message::EthMessage, types::hash::H512};

use super::Peer;

pub struct UnsafeSyncPtr<T> {
    pub(super) peer: *mut T,
}
unsafe impl<T> Sync for UnsafeSyncPtr<T> {}
unsafe impl<T> Send for UnsafeSyncPtr<T> {}

#[dynamic]
pub static PEERS_SELL: Mutex<HashMap<H512, UnsafeSyncPtr<Peer>>> = Mutex::new(HashMap::new());

impl Peer {
    pub async fn send_tx(msg: EthMessage) -> usize {
        let mut success_count: usize = 0;
        let start = tokio::time::Instant::now();

        let peers = PEERS_SELL.lock().await;
        let tasks = FuturesUnordered::new();

        let peers: Vec<&UnsafeSyncPtr<Peer>> = peers.values().collect();
        for chunk in peers.chunks(peers.len() / 4) {
            let chunk_futures = FuturesUnordered::from_iter(chunk.iter().map(|p| {
                let peer_ptr = unsafe { &mut p.peer.as_mut().unwrap().connection };
                let message = msg.clone(); // Assuming msg is defined elsewhere
                peer_ptr.send(message) // Assuming send() returns a Future
            }));

            let task = tokio::spawn(async move {
                let _ = chunk_futures.collect::<Vec<_>>().await;
            });
            tasks.push(task);
        }

        let results = tasks.collect::<Vec<_>>().await;
        println!("sending took: {:?}", start.elapsed());
        for t in results.iter() {
            match t {
                Ok(_) => {
                    success_count += 1;
                }
                _ => {} // Err(e) => {
                        //     cprintln!("<red>Send handle error: {e}</>",);
                        // }
            }
        }

        // let mut tasks = Vec::with_capacity(2_000);
        //
        // for peer in PEERS_SELL.lock().await.values() {
        //     let msg = msg.clone();
        //     let peer_ptr = unsafe { &mut peer.peer.as_mut().unwrap().connection };
        //     tasks.push(tokio::spawn(async move { peer_ptr.send(msg).await }));
        // }
        //
        // let results = join_all(tasks).await;
        // println!("sending took: {:?}", start.elapsed());
        // for t in results {
        //     match t {
        //         Ok(t) => match t {
        //             Ok(_) => {
        //                 success_count += 1;
        //             }
        //             _ => {} // Err(e) => {
        //                     //     cprintln!("<red>Send error: {e}</>",);
        //                     // }
        //         },
        //         _ => {} // Err(e) => {
        //                 //     cprintln!("<red>Send handle error: {e}</>",);
        //                 // }
        //     }
        // }
        // let tasks = FuturesUnordered::from_iter(PEERS_SELL.lock().await.iter().map(|(_, p)| {
        //     let peer_ptr = unsafe { &mut p.peer.as_mut().unwrap().connection };
        //     let message = msg.clone();
        //     tokio::spawn(async move { peer_ptr.send(message).await })
        // }));
        //
        //
        // let tasks = tasks.collect::<Vec<_>>().await;
        // println!("sending took: {:?}", start.elapsed());
        // for t in tasks {
        //     match t {
        //         Ok(t) => match t {
        //             Ok(_) => {
        //                 success_count += 1;
        //             }
        //             _ => {} // Err(e) => {
        //                     //     cprintln!("<red>Send error: {e}</>",);
        //                     // }
        //         },
        //         _ => {} // Err(e) => {
        //                 //     cprintln!("<red>Send handle error: {e}</>",);
        //                 // }
        //     }
        // }

        success_count
    }
}
