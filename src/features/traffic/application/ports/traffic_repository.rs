use async_trait::async_trait;
use crate::common::http::error::AppError;
use crate::features::traffic::domain::model::{TrafficPacket, TrafficSummary};

/// All database interactions for the `user_traffic_packets` table.
/// Implemented by `TrafficRepositoryImpl` in `common/app/adapters`.
#[async_trait]
pub trait TrafficRepository: Send + Sync {
    /// Returns the aggregated traffic summary (total + remaining) for a user across
    /// all active (non-expired) packets. Returns a zero summary if no packets exist.
    async fn get_summary(&self, user_id: &str) -> Result<TrafficSummary, AppError>;

    /// Adds a new traffic packet for a user with default 30 days validity.
    async fn add_packet(&self, user_id: &str, bytes: i64) -> Result<TrafficPacket, AppError>;

    /// Adds a new traffic packet for a user with custom validity duration in days.
    async fn add_packet_with_expiry(&self, user_id: &str, bytes: i64, duration_days: i64) -> Result<TrafficPacket, AppError>;

    /// Deducts `bytes` from the user's active packets in ascending remaining-traffic order.
    /// Returns the total remaining bytes after deduction.
    async fn consume(&self, user_id: &str, bytes: u64) -> Result<u64, AppError>;

    /// Returns the total remaining bytes across all active (non-expired) packets.
    async fn get_remaining(&self, user_id: &str) -> Result<u64, AppError>;

    /// Returns the history of all traffic packets for a user ordered by creation date (newest first).
    async fn get_history(&self, user_id: &str) -> Result<Vec<TrafficPacket>, AppError>;
}
