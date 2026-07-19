use serde::{Deserialize, Serialize};

use crate::features::nodes::domain::model::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<Node> for NodeDto {
    fn from(n: Node) -> Self {
        Self {
            id: n.id,
            name: n.name,
            status: n.status.to_string(),
            last_seen_at: n.last_seen_at,
        }
    }
}
