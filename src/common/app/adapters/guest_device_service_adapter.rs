use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    common::http::error::AppError,
    features::{
        abuse_shield::{
            application::commands::compute_device_key_hash,
            application::ports::DeviceIdentityRepository,
            domain::model::Platform,
        },
        auth::GuestDeviceService,
    },
};

pub struct GuestDeviceServiceAdapter {
    device_identity_repo: Arc<dyn DeviceIdentityRepository>,
    secret_salt: String,
}

impl GuestDeviceServiceAdapter {
    pub fn new(
        device_identity_repo: Arc<dyn DeviceIdentityRepository>,
        secret_salt: String,
    ) -> Self {
        Self {
            device_identity_repo,
            secret_salt,
        }
    }
}

#[async_trait]
impl GuestDeviceService for GuestDeviceServiceAdapter {
    async fn get_or_create_device_identity(
        &self,
        platform_str: &str,
        device_key: &str,
    ) -> Result<Uuid, AppError> {
        let platform: Platform = platform_str
            .parse()
            .map_err(|e| AppError::ValidationError(e))?;

        let device_key_hash = compute_device_key_hash(device_key, &self.secret_salt);

        let device = self
            .device_identity_repo
            .get_or_create_device_identity(platform, &device_key_hash)
            .await?;

        Ok(device.id)
    }
}
