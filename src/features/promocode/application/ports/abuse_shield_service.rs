use async_trait::async_trait;
use crate::common::http::error::AppError;

#[async_trait]
pub trait AbuseShieldService: Send + Sync {
    /// Checks if trial promo code was already redeemed on the device linked to user_id.
    async fn is_device_trial_redeemed(&self, user_id: &str) -> Result<bool, AppError>;

    /// Marks trial promo code as redeemed on the device linked to user_id.
    async fn mark_device_trial_redeemed(&self, user_id: &str) -> Result<(), AppError>;
}
