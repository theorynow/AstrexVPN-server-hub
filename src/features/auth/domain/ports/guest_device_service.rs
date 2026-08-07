use async_trait::async_trait;
use uuid::Uuid;
use crate::common::http::error::AppError;

#[async_trait]
pub trait GuestDeviceService: Send + Sync {
    async fn get_or_create_device_identity(&self, platform: &str, device_key: &str) -> Result<Uuid, AppError>;
}
