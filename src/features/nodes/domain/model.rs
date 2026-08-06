use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type NodeId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Online => write!(f, "online"),
            NodeStatus::Offline => write!(f, "offline"),
        }
    }
}

impl std::str::FromStr for NodeStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "online" => Ok(NodeStatus::Online),
            _ => Ok(NodeStatus::Offline),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XrayConfig {
    pub port: u16,
    pub sni: String,
    pub public_key: String,
    pub short_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HysteriaConfig {
    pub port: u16,
    pub sni: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub public_ip: String,
    pub name_en: String,
    pub country_code: String,
    pub country_flag: String,
    pub xray: Option<XrayConfig>,
    pub hysteria: Option<HysteriaConfig>,
    pub status: NodeStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TrafficReport {
    pub node_id: NodeId,
    pub user_bytes: HashMap<String, u64>,
}
