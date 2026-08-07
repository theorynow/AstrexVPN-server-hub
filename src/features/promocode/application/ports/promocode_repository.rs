use async_trait::async_trait;
use crate::{
    common::http::error::AppError,
    features::promocode::domain::model::{PromoCode, PromoCodeRewardType},
};

#[async_trait]
pub trait PromoCodeRepository: Send + Sync {
    /// Creates a new promo code.
    async fn create_promocode(
        &self,
        code: &str,
        reward_type: PromoCodeRewardType,
        reward_bytes: i64,
        duration_days: i32,
        created_by_user_id: Option<&str>,
        expires_in_days: i64,
    ) -> Result<PromoCode, AppError>;

    /// Finds a promo code by code string (case-insensitive).
    async fn find_by_code(&self, code: &str) -> Result<Option<PromoCode>, AppError>;

    /// Gets an active (unused & non-expired) trial promo code created by user, if any exists.
    async fn find_active_trial_for_creator(&self, user_id: &str) -> Result<Option<PromoCode>, AppError>;

    /// Checks if a user has already redeemed a reward of the specified type (e.g. TRIAL).
    async fn has_user_redeemed_reward_type(&self, user_id: &str, reward_type: PromoCodeRewardType) -> Result<bool, AppError>;

    /// Marks a promo code as redeemed by the given user.
    async fn mark_as_used(&self, code_id: &uuid::Uuid, user_id: &str) -> Result<(), AppError>;
}
