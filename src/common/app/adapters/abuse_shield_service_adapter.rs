use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        abuse_shield::application::ports::DeviceIdentityRepository,
        promocode::application::ports::AbuseShieldService,
    },
};

pub struct AbuseShieldServiceAdapter {
    device_identity_repo: Arc<dyn DeviceIdentityRepository>,
}

impl AbuseShieldServiceAdapter {
    pub fn new(device_identity_repo: Arc<dyn DeviceIdentityRepository>) -> Self {
        Self {
            device_identity_repo,
        }
    }
}

#[async_trait]
impl AbuseShieldService for AbuseShieldServiceAdapter {
    async fn is_device_trial_redeemed(&self, user_id: &str) -> Result<bool, AppError> {
        self.device_identity_repo
            .is_device_trial_redeemed_by_user(user_id)
            .await
    }

    async fn mark_device_trial_redeemed(&self, user_id: &str) -> Result<(), AppError> {
        self.device_identity_repo
            .mark_device_trial_redeemed_by_user(user_id)
            .await
    }
}
