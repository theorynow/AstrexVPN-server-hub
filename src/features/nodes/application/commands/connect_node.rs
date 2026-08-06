use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::domain::{
        model::{HysteriaConfig, Node, NodeStatus, XrayConfig},
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

    pub async fn execute(
        &self,
        node_id: &str,
        auth_secret: &str,
        public_ip: &str,
        name_en: &str,
        country_code: &str,
        country_flag: &str,
        xray: Option<XrayConfig>,
        hysteria: Option<HysteriaConfig>,
    ) -> Result<(), AppError> {
        if self.expected_secret != auth_secret {
            return Err(AppError::WrongCredentials);
        }

        let existing = self.repo.find_by_id(node_id).await?;
        let new_node = Node {
            id: node_id.to_string(),
            public_ip: public_ip.to_string(),
            name_en: name_en.to_string(),
            country_code: country_code.to_string(),
            country_flag: country_flag.to_string(),
            xray,
            hysteria,
            status: NodeStatus::Offline,
            last_seen_at: existing.as_ref().and_then(|n| n.last_seen_at),
            created_at: existing.as_ref().map(|n| n.created_at).unwrap_or_else(chrono::Utc::now),
            modified_at: chrono::Utc::now(),
        };
        self.repo.save(&new_node).await?;
        Ok(())
    }
}
