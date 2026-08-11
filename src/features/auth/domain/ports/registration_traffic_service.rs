use async_trait::async_trait;
use crate::common::http::error::AppError;

#[async_trait]
pub trait RegistrationTrafficService: Send + Sync {
    async fn grant_initial_traffic(&self, user_id: &str) -> Result<(), AppError>;
}
