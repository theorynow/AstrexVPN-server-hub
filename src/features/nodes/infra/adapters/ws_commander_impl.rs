use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::features::nodes::{
    api::dto::HubMessage,
    domain::ports::node_commander::{CommandResult, CommanderError, NodeCommander},
};

#[derive(Clone)]
pub struct ActiveNodeConnection {
    sender: mpsc::Sender<HubMessage>,
    inbound_tags: Vec<String>,
}

#[derive(Clone)]
pub struct WsCommanderImpl {
    active_nodes: Arc<RwLock<HashMap<String, ActiveNodeConnection>>>,
    pending_commands: Arc<Mutex<HashMap<String, oneshot::Sender<CommandResult>>>>,
}

impl Default for WsCommanderImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl WsCommanderImpl {
    pub fn new() -> Self {
        Self {
            active_nodes: Arc::new(RwLock::new(HashMap::new())),
            pending_commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl NodeCommander for WsCommanderImpl {
    fn register_node(
        &self,
        node_id: String,
        sender: mpsc::Sender<HubMessage>,
        inbound_tags: Vec<String>,
    ) {
        let mut active = self.active_nodes.write();
        active.insert(
            node_id,
            ActiveNodeConnection {
                sender,
                inbound_tags,
            },
        );
    }

    fn deregister_node(&self, node_id: &str) {
        let mut active = self.active_nodes.write();
        active.remove(node_id);
    }

    fn resolve_command(&self, command_id: &str, result: CommandResult) {
        let mut pending = self.pending_commands.lock();
        if let Some(tx) = pending.remove(command_id) {
            let _ = tx.send(result);
        }
    }

    async fn execute_add_user(
        &self,
        node_id: &str,
        user_uuid: &str,
    ) -> Result<bool, CommanderError> {
        let conn = {
            let active = self.active_nodes.read();
            active.get(node_id).cloned()
        };

        let conn = match conn {
            Some(c) => c,
            None => return Err(CommanderError::NodeNotConnected(node_id.to_string())),
        };

        let command_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_commands.lock();
            pending.insert(command_id.clone(), tx);
        }

        let cmd = HubMessage::AddUser {
            command_id: command_id.clone(),
            uuid: user_uuid.to_string(),
            inbound_tags: conn.inbound_tags.clone(),
        };

        if conn.sender.send(cmd).await.is_err() {
            let mut pending = self.pending_commands.lock();
            pending.remove(&command_id);
            return Err(CommanderError::SendError);
        }

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx).await;

        let response = match result {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => {
                return Err(CommanderError::Internal(
                    "Oneshot sender dropped".to_string(),
                ));
            }
            Err(_) => {
                let mut pending = self.pending_commands.lock();
                pending.remove(&command_id);
                return Err(CommanderError::Timeout);
            }
        };

        if response.success {
            Ok(true)
        } else {
            Err(CommanderError::NodeRejected(response.error_message))
        }
    }

    async fn execute_remove_user(
        &self,
        node_id: &str,
        user_uuid: &str,
    ) -> Result<bool, CommanderError> {
        let conn = {
            let active = self.active_nodes.read();
            active.get(node_id).cloned()
        };

        let conn = match conn {
            Some(c) => c,
            None => return Err(CommanderError::NodeNotConnected(node_id.to_string())),
        };

        let command_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_commands.lock();
            pending.insert(command_id.clone(), tx);
        }

        let cmd = HubMessage::RemoveUser {
            command_id: command_id.clone(),
            email: user_uuid.to_string(),
            inbound_tags: conn.inbound_tags.clone(),
        };

        if conn.sender.send(cmd).await.is_err() {
            let mut pending = self.pending_commands.lock();
            pending.remove(&command_id);
            return Err(CommanderError::SendError);
        }

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx).await;

        let response = match result {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => {
                return Err(CommanderError::Internal(
                    "Oneshot sender dropped".to_string(),
                ));
            }
            Err(_) => {
                let mut pending = self.pending_commands.lock();
                pending.remove(&command_id);
                return Err(CommanderError::Timeout);
            }
        };

        if response.success {
            Ok(true)
        } else {
            Err(CommanderError::NodeRejected(response.error_message))
        }
    }
}
