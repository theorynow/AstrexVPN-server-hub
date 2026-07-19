use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::domain::{
        model::{Node, NodeStatus},
        ports::node_repository::NodeRepository,
    },
};

pub struct ConnectNodeCommand {
    repo: Arc<dyn NodeRepository>,
    expected_secret: String,
}

impl ConnectNodeCommand {
    pub fn new(repo: Arc<dyn NodeRepository>, expected_secret: String) -> Self {
        Self {
            repo,
            expected_secret,
        }
    }

    pub async fn execute(&self, node_id: &str, auth_secret: &str) -> Result<(), AppError> {
        if self.expected_secret != auth_secret {
            return Err(AppError::WrongCredentials);
        }

        let node = self.repo.find_by_id(node_id).await?;
        if node.is_none() {
            // Auto-register new nodes if they don't exist yet
            let new_node = Node {
                id: node_id.to_string(),
                name: format!("Node {}", node_id),
                status: NodeStatus::Offline,
                last_seen_at: None,
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
            };
            self.repo.save(&new_node).await?;
        }
        Ok(())
    }
}
