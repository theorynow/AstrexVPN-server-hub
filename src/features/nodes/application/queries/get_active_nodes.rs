use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::domain::{model::Node, ports::node_repository::NodeRepository},
};

pub struct GetActiveNodesQuery {
    repo: Arc<dyn NodeRepository>,
}

impl GetActiveNodesQuery {
    pub fn new(repo: Arc<dyn NodeRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<Vec<Node>, AppError> {
        self.repo.get_active_nodes().await
    }
}
