use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::{
        nodes::application::ports::UserTrafficService,
        traffic::{
            application::ports::{TrafficRepository, RealtimePublisher},
            domain::model::{TrafficPacket, TrafficSummary},
        },
    },
};

#[derive(Clone)]
pub struct TrafficRepositoryImpl {
    pool: PgPool,
    publisher: Arc<dyn RealtimePublisher>,
}

impl TrafficRepositoryImpl {
    pub fn new(pool: PgPool, publisher: Arc<dyn RealtimePublisher>) -> Self {
        Self { pool, publisher }
    }

    async fn notify_traffic_change(&self, user_id: &str) -> Result<(), AppError> {
        let summary = self.get_summary(user_id).await?;
        let payload = serde_json::json!({
            "traffic_total_bytes": summary.total_bytes,
            "traffic_remaining_bytes": summary.remaining_bytes,
            "updated_at_ms": summary.updated_at_ms,
        });
        let channel = format!("personal:{}", user_id);
        if let Err(e) = self.publisher.publish(&channel, payload).await {
            tracing::error!("Centrifugo publish failed for channel {}: {:?}", channel, e);
        }
        Ok(())
    }
}

#[derive(Debug, FromRow)]
struct PacketRow {
    id: uuid::Uuid,
    traffic_remaining_bytes: i64,
}

#[derive(Debug, FromRow)]
struct TrafficPacketDb {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    traffic_limit_bytes: i64,
    traffic_remaining_bytes: i64,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
}

impl From<TrafficPacketDb> for TrafficPacket {
    fn from(db: TrafficPacketDb) -> Self {
        Self {
            id: db.id,
            user_id: db.user_id,
            traffic_limit_bytes: db.traffic_limit_bytes,
            traffic_remaining_bytes: db.traffic_remaining_bytes,
            expires_at: db.expires_at,
            created_at: db.created_at,
            modified_at: db.modified_at,
        }
    }
}

#[async_trait]
impl TrafficRepository for TrafficRepositoryImpl {
    async fn get_summary(&self, user_id: &str) -> Result<TrafficSummary, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        let row: Option<(Option<i64>, Option<i64>, Option<DateTime<Utc>>)> = sqlx::query_as(
            r#"
            SELECT 
                SUM(traffic_limit_bytes)::BIGINT,
                SUM(traffic_remaining_bytes)::BIGINT,
                MAX(modified_at)
            FROM user_traffic_packets
            WHERE user_id = $1 AND expires_at > now()
            "#
        )
        .bind(parsed_uuid)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((Some(total), Some(remaining), modified_at)) = row {
            let updated_at_ms = modified_at
                .unwrap_or_else(Utc::now)
                .timestamp_millis();
            Ok(TrafficSummary {
                total_bytes: total,
                remaining_bytes: remaining,
                updated_at_ms,
            })
        } else {
            Ok(TrafficSummary::default())
        }
    }

    async fn add_packet(&self, user_id: &str, bytes: i64) -> Result<TrafficPacket, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        let expires_at = Utc::now() + chrono::Duration::days(30);

        let db_packet = sqlx::query_as::<_, TrafficPacketDb>(
            r#"
            INSERT INTO user_traffic_packets (user_id, traffic_limit_bytes, traffic_remaining_bytes, expires_at)
            VALUES ($1, $2, $2, $3)
            RETURNING id, user_id, traffic_limit_bytes, traffic_remaining_bytes, expires_at, created_at, modified_at
            "#
        )
        .bind(parsed_uuid)
        .bind(bytes)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        let packet: TrafficPacket = db_packet.into();
        let _ = self.notify_traffic_change(user_id).await;
        Ok(packet)
    }

    async fn consume(&self, user_id: &str, bytes: u64) -> Result<u64, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        let mut tx = self.pool.begin().await?;

        // Select all active traffic packets of the user, ordered by remaining bytes ascending, locked FOR UPDATE
        let mut packets = sqlx::query_as::<_, PacketRow>(
            r#"
            SELECT id, traffic_remaining_bytes
            FROM user_traffic_packets
            WHERE user_id = $1 AND expires_at > now()
            ORDER BY traffic_remaining_bytes ASC
            FOR UPDATE
            "#
        )
        .bind(parsed_uuid)
        .fetch_all(&mut *tx)
        .await?;

        let mut remaining_to_deduct = bytes;

        for packet in &mut packets {
            if remaining_to_deduct == 0 {
                break;
            }
            let pkt_rem = packet.traffic_remaining_bytes.max(0) as u64;
            if pkt_rem >= remaining_to_deduct {
                let new_remaining = pkt_rem - remaining_to_deduct;
                sqlx::query(
                    r#"
                    UPDATE user_traffic_packets
                    SET traffic_remaining_bytes = $2, modified_at = now()
                    WHERE id = $1
                    "#
                )
                .bind(packet.id)
                .bind(new_remaining as i64)
                .execute(&mut *tx)
                .await?;
                packet.traffic_remaining_bytes = new_remaining as i64;
                remaining_to_deduct = 0;
            } else {
                remaining_to_deduct -= pkt_rem;
                sqlx::query(
                    r#"
                    UPDATE user_traffic_packets
                    SET traffic_remaining_bytes = 0, modified_at = now()
                    WHERE id = $1
                    "#
                )
                .bind(packet.id)
                .execute(&mut *tx)
                .await?;
                packet.traffic_remaining_bytes = 0;
            }
        }

        tx.commit().await?;

        let total_remaining: i64 = packets.iter().map(|p| p.traffic_remaining_bytes.max(0)).sum();
        let total_remaining_u64 = total_remaining as u64;
        let _ = self.notify_traffic_change(user_id).await;
        Ok(total_remaining_u64)
    }

    async fn get_remaining(&self, user_id: &str) -> Result<u64, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;
        
        let sum: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT SUM(traffic_remaining_bytes)::BIGINT
            FROM user_traffic_packets
            WHERE user_id = $1 AND expires_at > now()
            "#
        )
        .bind(parsed_uuid)
        .fetch_one(&self.pool)
        .await?;

        Ok(sum.unwrap_or(0).max(0) as u64)
    }
}

#[async_trait]
impl UserTrafficService for TrafficRepositoryImpl {
    async fn get_remaining_traffic(&self, user_uuid: &str) -> Result<u64, AppError> {
        self.get_remaining(user_uuid).await
    }

    async fn consume_traffic(&self, user_uuid: &str, bytes: u64) -> Result<u64, AppError> {
        self.consume(user_uuid, bytes).await
    }
}
