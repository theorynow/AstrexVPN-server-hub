use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommanderError {
    #[error("Node {0} is not connected")]
    NodeNotConnected(String),
    #[error("Node rejected the action: {0}")]
    NodeRejected(String),
    #[error("Command execution timed out")]
    Timeout,
    #[error("Failed to send command over WS channel")]
    SendError,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub error_message: String,
}

#[async_trait]
pub trait NodeCommander: Send + Sync {
    async fn execute_add_user(
        &self,
        node_id: &str,
        user_uuid: &str,
    ) -> Result<bool, CommanderError>;
    async fn execute_remove_user(
        &self,
        node_id: &str,
        user_uuid: &str,
    ) -> Result<bool, CommanderError>;

    fn register_node(
        &self,
        node_id: String,
        sender: tokio::sync::mpsc::Sender<crate::features::nodes::api::dto::HubMessage>,
        inbound_tags: Vec<String>,
    );
    fn deregister_node(&self, node_id: &str);
    fn resolve_command(&self, command_id: &str, result: CommandResult);
}
