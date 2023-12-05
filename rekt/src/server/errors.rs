use crate::rlpx::RLPXSessionError;

use super::connection_task::ConnectionTask;

pub struct ConnectionTaskError {
    pub conn_task: ConnectionTask,
    pub err: Option<RLPXSessionError>,
}

impl ConnectionTaskError {
    pub fn new(conn_task: ConnectionTask, err: RLPXSessionError) -> Self {
        Self {
            conn_task,
            err: Some(err),
        }
    }

    pub fn new_no_err(conn_task: ConnectionTask) -> Self {
        Self {
            conn_task,
            err: None,
        }
    }
}
