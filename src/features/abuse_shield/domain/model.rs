use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Android,
    Macos,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Macos => "macos",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "android" => Ok(Platform::Android),
            "macos" => Ok(Platform::Macos),
            _ => Err(format!("Unsupported platform: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    pub id: Uuid,
    pub registered_with_platform: Platform,
    pub device_key_hash: Vec<u8>,
    pub trial_redeemed_at: Option<DateTime<Utc>>,
}
