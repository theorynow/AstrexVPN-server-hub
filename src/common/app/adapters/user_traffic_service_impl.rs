use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use crate::{
    common::http::error::AppError,
    features::nodes::application::ports::UserTrafficService,
};

#[derive(Clone)]
pub struct UserTrafficServiceImpl {
    pool: PgPool,
}

impl UserTrafficServiceImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct PacketRow {
    id: uuid::Uuid,
    traffic_remaining_bytes: i64,
}

#[async_trait]
impl UserTrafficService for UserTrafficServiceImpl {
    async fn get_remaining_traffic(&self, user_uuid: &str) -> Result<u64, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_uuid)
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

        Ok(sum.unwrap_or(0) as u64)
    }

    async fn consume_traffic(&self, user_uuid: &str, bytes: u64) -> Result<u64, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_uuid)
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
        Ok(total_remaining as u64)
    }
}
