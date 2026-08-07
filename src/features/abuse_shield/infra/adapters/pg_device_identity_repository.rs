use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    common::http::error::AppError,
    features::abuse_shield::{
        application::ports::DeviceIdentityRepository,
        domain::model::{DeviceIdentity, Platform},
    },
};

pub struct PgDeviceIdentityRepository {
    pool: PgPool,
}

impl PgDeviceIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct DeviceIdentityDb {
    id: Uuid,
    registered_with_platform: String,
    device_key_hash: Vec<u8>,
    trial_redeemed_at: Option<DateTime<Utc>>,
}

impl From<DeviceIdentityDb> for DeviceIdentity {
    fn from(db: DeviceIdentityDb) -> Self {
        let platform = db
            .registered_with_platform
            .parse::<Platform>()
            .unwrap_or(Platform::Android);
        Self {
            id: db.id,
            registered_with_platform: platform,
            device_key_hash: db.device_key_hash,
            trial_redeemed_at: db.trial_redeemed_at,
        }
    }
}

#[async_trait]
impl DeviceIdentityRepository for PgDeviceIdentityRepository {
    async fn get_or_create_device_identity(
        &self,
        platform: Platform,
        device_key_hash: &[u8],
    ) -> Result<DeviceIdentity, AppError> {
        let db: DeviceIdentityDb = sqlx::query_as(
            r#"
            INSERT INTO device_identities (registered_with_platform, device_key_hash)
            VALUES ($1, $2)
            ON CONFLICT (registered_with_platform, device_key_hash)
            DO UPDATE SET registered_with_platform = EXCLUDED.registered_with_platform
            RETURNING id, registered_with_platform, device_key_hash, trial_redeemed_at
            "#
        )
        .bind(platform.as_str())
        .bind(device_key_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(db.into())
    }

    async fn is_device_trial_redeemed_by_user(&self, user_id: &str) -> Result<bool, AppError> {
        let parsed_uuid = Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        let res: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT (d.trial_redeemed_at IS NOT NULL) AS is_redeemed
            FROM users u
            JOIN device_identities d ON u.device_identity_id = d.id
            WHERE u.id = $1
            "#
        )
        .bind(parsed_uuid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(res.map(|r| r.0).unwrap_or(false))
    }

    async fn mark_device_trial_redeemed_by_user(&self, user_id: &str) -> Result<(), AppError> {
        let parsed_uuid = Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE device_identities d
            SET trial_redeemed_at = now()
            FROM users u
            WHERE u.device_identity_id = d.id AND u.id = $1
            "#
        )
        .bind(parsed_uuid)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
