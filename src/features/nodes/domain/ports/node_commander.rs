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
    #[error("Failed to send command over gRPC channel")]
    SendError,
    #[error("Internal error: {0}")]
    Internal(String),
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
        sender: tokio::sync::mpsc::Sender<
            Result<
                crate::features::nodes::api::grpc_codegen::vpn::infrastructure::HubCommand,
                tonic::Status,
            >,
        >,
        inbound_tags: Vec<String>,
    );
    fn deregister_node(&self, node_id: &str);
    fn resolve_command(
        &self,
        command_id: &str,
        result: crate::features::nodes::api::grpc_codegen::vpn::infrastructure::CommandResult,
    );
}
