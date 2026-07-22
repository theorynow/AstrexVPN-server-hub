use chrono::{DateTime, Utc};

use crate::{
    common::http::error::AppError,
    features::{traffic::domain::model::TrafficSummary, user::User},
};

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub traffic_total_bytes: i64,
    pub traffic_remaining_bytes: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
}

impl UserProfile {
    pub async fn resolve(user: User, summary: TrafficSummary) -> Result<Self, AppError> {
        let username = user
            .username
            .unwrap_or_else(|| format!("player-{}", user.id));
        Ok(Self {
            id: user.id,
            username,
            traffic_total_bytes: summary.total_bytes,
            traffic_remaining_bytes: summary.remaining_bytes,
            created_at: user.created_at,
            modified_at: user.modified_at,
        })
    }
}
