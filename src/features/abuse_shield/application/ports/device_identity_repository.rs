use async_trait::async_trait;
use crate::{
    common::http::error::AppError,
    features::abuse_shield::domain::model::{DeviceIdentity, Platform},
};

#[async_trait]
pub trait DeviceIdentityRepository: Send + Sync {
    /// Gets existing device identity by platform and device_key_hash, or creates a new one.
    async fn get_or_create_device_identity(
        &self,
        platform: Platform,
        device_key_hash: &[u8],
    ) -> Result<DeviceIdentity, AppError>;

    /// Checks if trial promo code was already redeemed on the device identity associated with the given user_id.
    async fn is_device_trial_redeemed_by_user(&self, user_id: &str) -> Result<bool, AppError>;

    /// Marks trial promo code as redeemed on the device identity associated with the given user_id.
    async fn mark_device_trial_redeemed_by_user(&self, user_id: &str) -> Result<(), AppError>;
}
