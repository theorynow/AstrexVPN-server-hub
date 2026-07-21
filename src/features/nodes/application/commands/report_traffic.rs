use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    features::nodes::{
        application::ports::UserTrafficService,
        domain::ports::node_commander::NodeCommander,
    },
};

pub struct ReportTrafficCommand {
    user_traffic_service: Arc<dyn UserTrafficService>,
    node_commander: Arc<dyn NodeCommander>,
}

impl ReportTrafficCommand {
    pub fn new(
        user_traffic_service: Arc<dyn UserTrafficService>,
        node_commander: Arc<dyn NodeCommander>,
    ) -> Self {
        Self {
            user_traffic_service,
            node_commander,
        }
    }

    pub async fn execute(&self, node_id: &str, user_bytes: HashMap<String, u64>) {
        for (user_uuid, bytes) in user_bytes {
            tracing::info!(
                node_id = %node_id,
                user_uuid = %user_uuid,
                bytes_transferred = bytes,
                "Node traffic report received"
            );

            match self.user_traffic_service.consume_traffic(&user_uuid, bytes).await {
                Ok(remaining) => {
                    tracing::info!(
                        user_uuid = %user_uuid,
                        remaining_bytes = remaining,
                        "Successfully consumed traffic"
                    );

                    if remaining == 0 {
                        tracing::warn!(
                            user_uuid = %user_uuid,
                            node_id = %node_id,
                            "User has exhausted their traffic. Removing from node."
                        );

                        if let Err(e) = self.node_commander.execute_remove_user(node_id, &user_uuid).await {
                            tracing::error!(
                                node_id = %node_id,
                                user_uuid = %user_uuid,
                                error = %e,
                                "Failed to send remove user command to node after traffic exhaustion"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        user_uuid = %user_uuid,
                        error = %e,
                        "Failed to update traffic in DB"
                    );
                }
            }
        }
    }
}
