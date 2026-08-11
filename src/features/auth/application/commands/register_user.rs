use std::sync::Arc;

use crate::{
    common::{http::error::AppError, security::hash_util},
    features::auth::{AuthRepository, RegisterUser, RegistrationTrafficService},
};

pub struct RegisterUserCommand {
    repo: Arc<dyn AuthRepository>,
    registration_traffic_service: Arc<dyn RegistrationTrafficService>,
}

impl RegisterUserCommand {
    pub fn new(
        repo: Arc<dyn AuthRepository>,
        registration_traffic_service: Arc<dyn RegistrationTrafficService>,
    ) -> Self {
        Self {
            repo,
            registration_traffic_service,
        }
    }

    pub async fn execute(&self, input: RegisterUser) -> Result<String, AppError> {
        let username = input.username.trim().to_string();
        if username.is_empty() {
            return Err(AppError::ValidationError("Username cannot be empty".into()));
        }

        let password_hash = match input.password {
            Some(ref password) => {
                let trimmed = password.trim();
                if trimmed.is_empty() {
                    return Err(AppError::ValidationError("Password cannot be empty".into()));
                }
                Some(hash_util::hash_password(trimmed).map_err(|_| AppError::InternalError)?)
            }
            None => None,
        };

        let user_id = self
            .repo
            .create_user_with_auth(Some(username), password_hash)
            .await?;

        self.registration_traffic_service
            .grant_initial_traffic(&user_id)
            .await?;

        Ok(user_id)
    }
}
