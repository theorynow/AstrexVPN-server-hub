use std::sync::Arc;
use uuid::Uuid;

use crate::{
    common::{http::error::AppError, security::jwt::AuthBody},
    features::auth::{
        application::commands::session_tokens::issue_tokens_with_family, AuthRepository,
        GuestDeviceService, RegistrationTrafficService,
    },
};

pub struct AuthAsGuestCommand {
    repo: Arc<dyn AuthRepository>,
    guest_device_service: Arc<dyn GuestDeviceService>,
    registration_traffic_service: Arc<dyn RegistrationTrafficService>,
}

impl AuthAsGuestCommand {
    pub fn new(
        repo: Arc<dyn AuthRepository>,
        guest_device_service: Arc<dyn GuestDeviceService>,
        registration_traffic_service: Arc<dyn RegistrationTrafficService>,
    ) -> Self {
        Self {
            repo,
            guest_device_service,
            registration_traffic_service,
        }
    }

    pub async fn execute(&self, device_key: &str, platform: &str) -> Result<AuthBody, AppError> {
        let device_identity_id = self
            .guest_device_service
            .get_or_create_device_identity(platform, device_key)
            .await?;

        let tentative_id = Uuid::new_v4();
        let username = format!("guest-{}", tentative_id);

        let user_id = self
            .repo
            .create_user_with_auth_and_device(Some(username), None, Some(device_identity_id))
            .await?;

        self.registration_traffic_service
            .grant_initial_traffic(&user_id)
            .await?;

        issue_tokens_with_family(&self.repo, user_id, Uuid::new_v4()).await
    }
}
