use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::application::ports::UserTrafficService,
    features::nodes::domain::ports::node_commander::{CommanderError, NodeCommander},
};

pub struct AddUserToNodeCommand<C: NodeCommander + ?Sized> {
    commander: Arc<C>,
    user_traffic_service: Arc<dyn UserTrafficService>,
}

impl<C: NodeCommander + ?Sized> AddUserToNodeCommand<C> {
    pub fn new(commander: Arc<C>, user_traffic_service: Arc<dyn UserTrafficService>) -> Self {
        Self {
            commander,
            user_traffic_service,
        }
    }

    pub async fn execute(&self, node_id: &str, user_uuid: &str) -> Result<(), AppError> {
        // Check user remaining traffic. Reject if it is 0.
        let remaining = self.user_traffic_service.get_remaining_traffic(user_uuid).await?;
        if remaining == 0 {
            return Err(AppError::TrafficExhausted);
        }

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
