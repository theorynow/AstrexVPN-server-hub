use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::{
    common::http::error::AppError,
    features::promocode::{
        application::ports::PromoCodeRepository,
        domain::model::{PromoCode, PromoCodeRewardType},
    },
};

#[derive(Clone)]
pub struct PgPromoCodeRepository {
    pool: PgPool,
}

impl PgPromoCodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct PromoCodeDb {
    id: uuid::Uuid,
    code: String,
    reward_type: String,
    reward_bytes: i64,
    duration_days: i32,
    created_by_user_id: Option<uuid::Uuid>,
    used_by_user_id: Option<uuid::Uuid>,
    expires_at: DateTime<Utc>,
    used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<PromoCodeDb> for PromoCode {
    fn from(db: PromoCodeDb) -> Self {
        let reward_type = db
            .reward_type
            .parse::<PromoCodeRewardType>()
            .unwrap_or(PromoCodeRewardType::Trial);
        Self {
            id: db.id,
            code: db.code,
            reward_type,
            reward_bytes: db.reward_bytes,
            duration_days: db.duration_days,
            created_by_user_id: db.created_by_user_id,
            used_by_user_id: db.used_by_user_id,
            expires_at: db.expires_at,
            used_at: db.used_at,
            created_at: db.created_at,
        }
    }
}

#[async_trait]
impl PromoCodeRepository for PgPromoCodeRepository {
    async fn create_promocode(
        &self,
        code: &str,
        reward_type: PromoCodeRewardType,
        reward_bytes: i64,
        duration_days: i32,
        created_by_user_id: Option<&str>,
        expires_in_days: i64,
    ) -> Result<PromoCode, AppError> {
        let creator_uuid = match created_by_user_id {
            Some(id) => Some(
                uuid::Uuid::parse_str(id)
                    .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?,
            ),
            None => None,
        };

        let expires_at = Utc::now() + chrono::Duration::days(expires_in_days);

        let db = sqlx::query_as::<_, PromoCodeDb>(
            r#"
            INSERT INTO promocodes (code, reward_type, reward_bytes, duration_days, created_by_user_id, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, code, reward_type, reward_bytes, duration_days, created_by_user_id, used_by_user_id, expires_at, used_at, created_at
            "#
        )
        .bind(code.to_uppercase())
        .bind(reward_type.as_str())
        .bind(reward_bytes)
        .bind(duration_days)
        .bind(creator_uuid)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(db.into())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<PromoCode>, AppError> {
        let db_opt = sqlx::query_as::<_, PromoCodeDb>(
            r#"
            SELECT id, code, reward_type, reward_bytes, duration_days, created_by_user_id, used_by_user_id, expires_at, used_at, created_at
            FROM promocodes
            WHERE UPPER(code) = UPPER($1)
            "#
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(db_opt.map(Into::into))
    }

    async fn find_active_trial_for_creator(&self, user_id: Option<&str>) -> Result<Option<PromoCode>, AppError> {
        let creator_uuid = match user_id {
            Some(id) => uuid::Uuid::parse_str(id)
                .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?,
            None => return Ok(None),
        };

        let db_opt = sqlx::query_as::<_, PromoCodeDb>(
            r#"
            SELECT id, code, reward_type, reward_bytes, duration_days, created_by_user_id, used_by_user_id, expires_at, used_at, created_at
            FROM promocodes
            WHERE created_by_user_id = $1 AND reward_type = 'TRIAL' AND used_at IS NULL AND expires_at > now()
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(creator_uuid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(db_opt.map(Into::into))
    }

    async fn count_user_redeemed_reward_type(
        &self,
        user_id: &str,
        reward_type: PromoCodeRewardType,
    ) -> Result<i64, AppError> {
        let parsed_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM promocodes
            WHERE used_by_user_id = $1 AND reward_type = $2 AND used_at IS NOT NULL
            "#
        )
        .bind(parsed_uuid)
        .bind(reward_type.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    async fn mark_as_used(&self, code_id: &uuid::Uuid, user_id: &str) -> Result<(), AppError> {
        let parsed_user_uuid = uuid::Uuid::parse_str(user_id)
            .map_err(|e| AppError::ValidationError(format!("Invalid UUID format: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE promocodes
            SET used_by_user_id = $2, used_at = now()
            WHERE id = $1
            "#
        )
        .bind(code_id)
        .bind(parsed_user_uuid)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
