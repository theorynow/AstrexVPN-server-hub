use chrono::{DateTime, Utc};

/// Represents a single traffic allocation packet (e.g., 25 GB for 30 days).
#[derive(Debug, Clone)]
pub struct TrafficPacket {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub traffic_limit_bytes: i64,
    pub traffic_remaining_bytes: i64,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// Aggregated traffic summary for a user across all active packets.
#[derive(Debug, Clone, Default)]
pub struct TrafficSummary {
    /// Total bytes across all active non-expired packets (the "plan" size).
    pub total_bytes: i64,
    /// Remaining bytes across all active non-expired packets.
    pub remaining_bytes: i64,
    /// Unix timestamp (milliseconds) of the last change to any active packet.
    /// Used as a monotonic cursor by clients to resolve REST vs WS ordering:
    /// discard any update where updated_at_ms <= last_applied_at_ms.
    pub updated_at_ms: i64,
}

