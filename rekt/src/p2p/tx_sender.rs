use std::{collections::HashMap, usize};

use color_print::cprintln;
use futures::SinkExt;
use static_init::dynamic;
use tokio::sync::Mutex;

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
        let start = std::time::Instant::now();
        let mut tasks = Vec::with_capacity(500);
        for (_, p) in PEERS_SELL.lock().await.iter() {
            let peer_ptr = unsafe { &mut p.peer.as_mut().unwrap().connection };
            let message = msg.clone();
            let t = tokio::task::spawn(async move {
                match peer_ptr.send(message).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        cprintln!("<red>Buy error: {e}</>",);
                        Err(e)
                    }
                }
            });
            tasks.push(t);
        }

        let tasks = futures::future::join_all(tasks).await;
        println!("sending took: {:?}", start.elapsed());
        for t in tasks {
            match t {
                Ok(t) => match t {
                    Ok(_) => {
                        success_count += 1;
                    }
                    Err(e) => {
                        cprintln!("<red>Buy error: {e}</>",);
                    }
                },
                Err(e) => {
                    cprintln!("<red>Buy handle error: {e}</>",);
                }
            }
        }

        success_count
    }
}
