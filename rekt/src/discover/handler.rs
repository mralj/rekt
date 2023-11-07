use std::{collections::HashMap, sync::Arc};

use futures::{stream::FuturesUnordered, StreamExt};

use crate::{
    blockchain::bsc_chain_spec::BSC_MAINNET_FORK_FILTER, rlpx::Connection,
    server::connection_task::ConnectionTask, types::hash::H512,
};

use super::{
    discover_node::{AuthStatus, DiscoverNode},
    messages::{
        decoded_discover_message::DecodedDiscoverMessage, discover_message::DiscoverMessage,
        enr::EnrResponse, lookup::PendingNeighboursReq, ping_pong_messages::PongMessage,
    },
    server::Server,
};

impl Server {
    pub(super) async fn handle_received_msg(this: Arc<Self>, msg: DecodedDiscoverMessage) {
        match msg.msg {
            DiscoverMessage::Ping(ping) => {
                match this.nodes.entry(msg.node_id) {
                    dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                        entry.get_mut().mark_ping_received();
                    }
                    dashmap::mapref::entry::Entry::Vacant(entry) => {
                        if let Ok(node) = DiscoverNode::from_ping_msg(&ping, msg.node_id) {
                            entry.insert(node);
                        }
                    }
                };

                let pong = DiscoverMessage::Pong(PongMessage::new(ping, msg.hash));
                let packet =
                    DiscoverMessage::create_disc_v4_packet(pong, &this.local_node.private_key);
                let _ = this.udp_sender.send((msg.from, packet)).await;

                //TODO: if we received pig from node that has pending lookup
                // and this node is not authed we can now send find node message
                // as will it be authed after pong message we just sent
                if let Some(req) = this.pending_neighbours_req.get(&msg.node_id) {
                    if req.was_authed {
                        return;
                    }

                    this.send_neighbours_packet(req.lookup_id, (req.ip, req.udp))
                        .await;
                }
            }
            DiscoverMessage::Pong(_) => {
                this.pending_pings.remove(&msg.node_id);
                if let Some(node) = &mut this.nodes.get_mut(&msg.node_id) {
                    node.mark_pong_received();
                }
            }
            DiscoverMessage::EnrRequest(_) => {
                let enr_response = DiscoverMessage::EnrResponse(EnrResponse::new(
                    msg.hash,
                    this.local_node.enr.clone(),
                ));
                let packet = DiscoverMessage::create_disc_v4_packet(
                    enr_response,
                    &this.local_node.private_key,
                );
                let _ = this.udp_sender.send((msg.from, packet)).await;
            }
            DiscoverMessage::EnrResponse(resp) => {
                //TODO: IMPLEMENT THIS
                //
                let forks_match = {
                    if let Some(fork_id) = resp.eth_fork_id() {
                        BSC_MAINNET_FORK_FILTER.validate(fork_id).is_ok()
                    } else {
                        false
                    }
                };

                if let Some(node) = &mut this.nodes.get_mut(&msg.node_id) {
                    node.set_is_bsc(forks_match);

                    if forks_match {
                        let conn_task =
                            ConnectionTask::new_from_node_record(node.node_record.clone());
                        let _ = this.conn_tx.send(conn_task).await;
                    }
                }
            }
            DiscoverMessage::Neighbours(neighbours) => {
                let req = this.pending_neighbours_req.remove(&msg.node_id);
                if req.is_none() {
                    return;
                }
                let req = req.unwrap().1;
                if let Some(lookup) = &mut this.pending_lookups.get_mut(&req.lookup_id) {
                    lookup.mark_node_responded(msg.node_id);

                    let nodes = neighbours
                        .nodes
                        .into_iter()
                        .filter_map(|node| DiscoverNode::try_from(node).ok())
                        .map(|node| (node.id(), node))
                        .collect::<HashMap<H512, DiscoverNode>>();

                    let mut all_nodes = Vec::with_capacity(nodes.len());
                    let mut unknown_nodes = Vec::with_capacity(nodes.len());

                    for (id, node) in nodes.into_iter() {
                        if let Some(already_known_node) = this.nodes.get(&id) {
                            all_nodes.push(already_known_node.clone());
                        } else {
                            unknown_nodes.push(node.clone());
                            all_nodes.push(node);
                        }
                    }

                    unknown_nodes.into_iter().for_each(|n| {
                        this.nodes.insert(n.id(), n);
                    });

                    lookup.add_new_nodes(all_nodes);

                    let nex_query_batch = lookup.get_next_nodes_to_query(&this.nodes);
                    nex_query_batch.iter().for_each(|n| {
                        this.pending_neighbours_req
                            .insert(n.id(), PendingNeighboursReq::new(req.lookup_id, n));
                    });

                    //NOTE: for unauthed nodes we send ping message "in hope of" following
                    //happenig:
                    // 1. we send ping message
                    // 2. node is "live" and it sends pong back (less important for neighbours
                    //    message, but important for obtainging new BSC nodes)
                    // 3. node sends US ping message (to which we respond with pong)
                    // 4. now this node considers us authed and we can send find_node message
                    // I say "in hope of" because we can't be sure that node will send us ping
                    let tasks = FuturesUnordered::from_iter(
                        nex_query_batch
                            .iter()
                            .filter(|n| n.auth_status() == AuthStatus::NotAuthed)
                            .map(|n| {
                                this.send_ping_packet((
                                    n.id(),
                                    n.node_record.clone(),
                                    n.ip_v4_addr,
                                    n.udp_port(),
                                ))
                            }),
                    );
                    let _result = tasks.collect::<Vec<_>>().await;

                    let tasks = FuturesUnordered::from_iter(
                        nex_query_batch
                            .iter()
                            .filter(|n| {
                                n.auth_status() == AuthStatus::Authed
                                    || n.auth_status() == AuthStatus::TheyAuthedUs
                            })
                            .map(|n| {
                                this.send_neighbours_packet(
                                    req.lookup_id,
                                    (n.ip_v4_addr, n.udp_port()),
                                )
                            }),
                    );

                    let _result = tasks.collect::<Vec<_>>().await;
                } else {
                    println!("Unknown lookup");
                }
            }
            _ => {}
        }
    }
}
