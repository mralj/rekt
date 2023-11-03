use super::{
    discover_node::DiscoverNode,
    messages::{
        decoded_discover_message::DecodedDiscoverMessage, discover_message::DiscoverMessage,
        enr::EnrResponse, ping_pong_messages::PongMessage,
    },
    server::Server,
};

impl Server {
    pub(super) async fn handle_received_msg(&self, msg: DecodedDiscoverMessage) {
        match msg.msg {
            DiscoverMessage::Ping(ping) => {
                match self.nodes.entry(msg.node_id) {
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
                    DiscoverMessage::create_disc_v4_packet(pong, &self.local_node.private_key);
                let _ = self.udp_sender.send((msg.from, packet)).await;
            }
            DiscoverMessage::Pong(_) => {
                self.pending_pings.remove(&msg.node_id);
                if let Some(node) = &mut self.nodes.get_mut(&msg.node_id) {
                    node.mark_pong_received();
                }
            }
            DiscoverMessage::EnrRequest(_) => {
                let enr_response = DiscoverMessage::EnrResponse(EnrResponse::new(
                    msg.hash,
                    self.local_node.enr.clone(),
                ));
                let packet = DiscoverMessage::create_disc_v4_packet(
                    enr_response,
                    &self.local_node.private_key,
                );
                let _ = self.udp_sender.send((msg.from, packet)).await;
            }
            DiscoverMessage::EnrResponse(_) => {
                //TODO: IMPLEMENT THIS
                //
                // let forks_match = {
                //     if let Some(fork_id) = enr_response.eth_fork_id() {
                //         BSC_MAINNET_FORK_FILTER.validate(fork_id).is_ok()
                //     } else {
                //         false
                //     }
                // };
                // println!(
                //     "[{}] ENR Response message [{:?}]: {:?}, is match: {}",
                //     now, src, enr_response, forks_match
                // );
            }
            DiscoverMessage::Neighbours(_) => {
                //TODO: IMPLEMENT THIS
            }
            _ => {}
        }
    }
}
