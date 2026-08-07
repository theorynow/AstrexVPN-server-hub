use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetTrafficDto {
    pub user_id: Option<String>,
    pub mb: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrafficPacketDto {
    pub id: String,
    pub user_id: String,
    pub traffic_limit_bytes: i64,
    pub traffic_remaining_bytes: i64,
    pub created_at: String,
    pub expires_at: String,
}

impl From<crate::features::traffic::domain::model::TrafficPacket> for TrafficPacketDto {
    fn from(p: crate::features::traffic::domain::model::TrafficPacket) -> Self {
        Self {
            id: p.id.to_string(),
            user_id: p.user_id.to_string(),
            traffic_limit_bytes: p.traffic_limit_bytes,
            traffic_remaining_bytes: p.traffic_remaining_bytes,
            created_at: p.created_at.to_rfc3339(),
            expires_at: p.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CentrifugeTokenDto {
    pub connection_token: String,
    pub subscription_token: String,
    pub channel: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrafficSummaryDto {
    pub traffic_total_bytes: i64,
    pub traffic_remaining_bytes: i64,
    /// Unix timestamp in milliseconds of the last traffic change.
    /// Clients must use this as a monotonic cursor:
    /// discard any incoming update (REST or WS) if its `updated_at_ms`
    /// is less than or equal to the last applied value.
    pub updated_at_ms: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrafficHistoryItemDto {
    pub id: String,
    pub traffic_limit_bytes: i64,
    pub traffic_remaining_bytes: i64,
    pub created_at: String,
    pub expires_at: String,
}

impl From<crate::features::traffic::domain::model::TrafficPacket> for TrafficHistoryItemDto {
    fn from(p: crate::features::traffic::domain::model::TrafficPacket) -> Self {
        Self {
            id: p.id.to_string(),
            traffic_limit_bytes: p.traffic_limit_bytes,
            traffic_remaining_bytes: p.traffic_remaining_bytes,
            created_at: p.created_at.to_rfc3339(),
            expires_at: p.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrafficHistoryResponseDto {
    pub server_time: String,
    pub items: Vec<TrafficHistoryItemDto>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::features::traffic::domain::model::TrafficPacket;

    #[test]
    fn test_traffic_history_item_dto_conversion_and_serialization() {
        let packet = TrafficPacket {
            id: Uuid::nil(),
            user_id: Uuid::new_v4(),
            traffic_limit_bytes: 1000,
            traffic_remaining_bytes: 800,
            expires_at: Utc::now(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        let item: TrafficHistoryItemDto = packet.into();
        let response = TrafficHistoryResponseDto {
            server_time: Utc::now().to_rfc3339(),
            items: vec![item],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("server_time"));
        assert!(json.contains("traffic_limit_bytes"));
        assert!(json.contains("traffic_remaining_bytes"));
        assert!(json.contains("created_at"));
        assert!(json.contains("expires_at"));
        assert!(!json.contains("user_id"));
    }
}
