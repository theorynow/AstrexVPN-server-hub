use crate::common::http::error::AppError;
use crate::features::nodes::domain::model::{Node, NodeStatus};
use async_trait::async_trait;

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn save(&self, node: &Node) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Node>, AppError>;
    async fn update_status(&self, id: &str, status: NodeStatus) -> Result<(), AppError>;
    async fn get_active_nodes(&self) -> Result<Vec<Node>, AppError>;
}
