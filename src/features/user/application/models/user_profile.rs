use chrono::{DateTime, Utc};

use crate::{common::http::error::AppError, features::user::User};

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub traffic_limit_bytes: i64,
    pub traffic_used_bytes: i64,
    pub remaining_traffic_bytes: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
}

impl UserProfile {
    pub async fn resolve(user: User) -> Result<Self, AppError> {
        let username = user
            .username
            .unwrap_or_else(|| format!("player-{}", user.id));
        let remaining_traffic_bytes =
            std::cmp::max(0, user.traffic_limit_bytes - user.traffic_used_bytes);
        Ok(Self {
            id: user.id,
            username,
            traffic_limit_bytes: user.traffic_limit_bytes,
            traffic_used_bytes: user.traffic_used_bytes,
            remaining_traffic_bytes,
            created_at: user.created_at,
            modified_at: user.modified_at,
        })
    }
}
