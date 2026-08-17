use crate::common::http::error::AppError;
use crate::features::nodes::domain::model::{Node, NodeStatus};
use async_trait::async_trait;

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn save(&self, node: &Node) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Node>, AppError>;
    async fn update_status(&self, id: &str, status: NodeStatus) -> Result<(), AppError>;
    async fn get_active_nodes(&self) -> Result<Vec<Node>, AppError>;
    /// Upsert node data on connect — single query, preserves existing created_at/last_seen_at.
    async fn upsert_on_connect(
        &self,
        id: &str,
        public_ip: &str,
        name_en: &str,
        country_code: &str,
        country_flag: &str,
        xray: Option<&crate::features::nodes::domain::model::XrayConfig>,
        hysteria: Option<&crate::features::nodes::domain::model::HysteriaConfig>,
    ) -> Result<(), AppError>;
}

