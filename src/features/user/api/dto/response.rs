use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::features::user::UserProfile;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub traffic_total_bytes: i64,
    pub traffic_remaining_bytes: i64,
    #[serde(with = "crate::common::serde::ts_format::option")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::common::serde::ts_format::option")]
    pub modified_at: Option<DateTime<Utc>>,
}

impl From<UserProfile> for UserDto {
    fn from(user: UserProfile) -> Self {
        Self {
            id: user.id,
            username: user.username,
            traffic_total_bytes: user.traffic_total_bytes,
            traffic_remaining_bytes: user.traffic_remaining_bytes,
            created_at: user.created_at,
            modified_at: user.modified_at,
        }
    }
}
