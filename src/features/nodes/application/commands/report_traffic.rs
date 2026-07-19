use sqlx::PgPool;
use std::collections::HashMap;

pub struct ReportTrafficCommand {
    pool: PgPool,
}

impl ReportTrafficCommand {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn execute(&self, node_id: &str, user_bytes: HashMap<String, u64>) {
        for (user_uuid, bytes) in user_bytes {
            tracing::info!(
                node_id = %node_id,
                user_uuid = %user_uuid,
                bytes_transferred = bytes,
                "Node traffic report received"
            );

            if let Ok(parsed_uuid) = uuid::Uuid::parse_str(&user_uuid) {
                let query_res = sqlx::query(
                    r#"
                    UPDATE users
                    SET 
                        traffic_used_bytes = traffic_used_bytes + $2,
                        modified_at = now()
                    WHERE id = $1
                    "#,
                )
                .bind(parsed_uuid)
                .bind(bytes as i64)
                .execute(&self.pool)
                .await;

                if let Err(e) = query_res {
                    tracing::error!(
                        user_uuid = %user_uuid,
                        error = %e,
                        "Failed to update traffic in DB"
                    );
                }
            } else {
                tracing::warn!(
                    user_uuid = %user_uuid,
                    "Skipping traffic update: invalid UUID format"
                );
            }
        }
    }
}
