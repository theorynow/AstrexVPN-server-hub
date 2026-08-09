use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromoCodeRewardType {
    Trial,
}

impl PromoCodeRewardType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trial => "TRIAL",
        }
    }
}

impl fmt::Display for PromoCodeRewardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PromoCodeRewardType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRIAL" => Ok(Self::Trial),
            _ => Err(format!("Unknown reward type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromoCode {
    pub id: uuid::Uuid,
    pub code: String,
    pub reward_type: PromoCodeRewardType,
    pub reward_bytes: i64,
    pub duration_days: i32,
    pub max_uses: i32,
    pub current_uses: i32,
    pub created_by_user_id: Option<uuid::Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl PromoCode {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    pub fn is_used(&self) -> bool {
        self.current_uses >= self.max_uses
    }
}
