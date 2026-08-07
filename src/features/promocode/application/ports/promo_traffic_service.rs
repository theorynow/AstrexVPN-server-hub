use async_trait::async_trait;
use crate::common::http::error::AppError;

#[async_trait]
pub trait PromoTrafficService: Send + Sync {
    async fn grant_traffic(&self, user_id: &str, bytes: i64, duration_days: i64) -> Result<(), AppError>;
}
