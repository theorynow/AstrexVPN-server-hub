use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddTrafficDto {
    pub user_id: String,
    pub mb: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubtractTrafficDto {
    pub user_id: String,
    pub mb: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetTrafficDto {
    pub user_id: String,
    pub mb: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrafficPacketDto {
    pub id: String,
    pub user_id: String,
    pub traffic_limit_bytes: i64,
    pub traffic_remaining_bytes: i64,
    pub expires_at: String,
}

impl From<crate::features::traffic::domain::model::TrafficPacket> for TrafficPacketDto {
    fn from(p: crate::features::traffic::domain::model::TrafficPacket) -> Self {
        Self {
            id: p.id.to_string(),
            user_id: p.user_id.to_string(),
            traffic_limit_bytes: p.traffic_limit_bytes,
            traffic_remaining_bytes: p.traffic_remaining_bytes,
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
