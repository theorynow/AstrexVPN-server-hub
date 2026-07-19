use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::domain::ports::node_commander::{CommanderError, NodeCommander},
};

pub struct AddUserToNodeCommand<C: NodeCommander + ?Sized> {
    commander: Arc<C>,
}

impl<C: NodeCommander + ?Sized> AddUserToNodeCommand<C> {
    pub fn new(commander: Arc<C>) -> Self {
        Self { commander }
    }

    pub async fn execute(&self, node_id: &str, user_uuid: &str) -> Result<(), AppError> {
        let success = self
            .commander
            .execute_add_user(node_id, user_uuid)
            .await
            .map_err(|e| match e {
                CommanderError::NodeNotConnected(_) => AppError::ValidationError(e.to_string()),
                CommanderError::NodeRejected(err_msg) => {
                    AppError::ValidationError(format!("Node rejected: {}", err_msg))
                }
                CommanderError::Timeout => {
                    AppError::ValidationError("Timeout waiting for node".to_string())
                }
                _ => AppError::InternalError,
            })?;

        if !success {
            return Err(AppError::NodeRejectedAction);
        }
        Ok(())
    }
}
