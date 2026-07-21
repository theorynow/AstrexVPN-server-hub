use async_trait::async_trait;
use crate::common::http::error::AppError;

#[async_trait]
pub trait UserTrafficService: Send + Sync {
    async fn get_remaining_traffic(&self, user_uuid: &str) -> Result<u64, AppError>;
    async fn consume_traffic(&self, user_uuid: &str, bytes: u64) -> Result<u64, AppError>;
}
