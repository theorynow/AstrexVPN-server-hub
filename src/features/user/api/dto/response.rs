use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::features::user::UserProfile;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub traffic_limit_bytes: i64,
    pub traffic_used_bytes: i64,
    pub remaining_traffic_bytes: i64,
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
            traffic_limit_bytes: user.traffic_limit_bytes,
            traffic_used_bytes: user.traffic_used_bytes,
            remaining_traffic_bytes: user.remaining_traffic_bytes,
            created_at: user.created_at,
            modified_at: user.modified_at,
        }
    }
}
