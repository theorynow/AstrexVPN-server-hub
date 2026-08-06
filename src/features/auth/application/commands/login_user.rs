use std::sync::Arc;

use crate::{
    common::{
        http::error::AppError,
        security::{hash_util, jwt::AuthBody},
    },
    features::auth::{
        application::commands::session_tokens::issue_tokens_with_family, AuthRepository, LoginUser,
    },
};

pub struct LoginUserCommand {
    repo: Arc<dyn AuthRepository>,
}

impl LoginUserCommand {
    pub fn new(repo: Arc<dyn AuthRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: LoginUser) -> Result<AuthBody, AppError> {
        let username = input.username.trim();
        let password = input.password.as_deref().map(|p| p.trim()).unwrap_or("");

        if username.is_empty() || password.is_empty() {
            return Err(AppError::MissingCredentials);
        }

        let user_auth = self
            .repo
            .find_by_username(username)
            .await?
            .ok_or(AppError::UserNotFound)?;

        let stored_hash = user_auth
            .password_hash
            .as_deref()
            .ok_or(AppError::WrongCredentials)?;

        if !hash_util::verify_password(stored_hash, password) {
            return Err(AppError::WrongCredentials);
        }

        issue_tokens_with_family(&self.repo, user_auth.user_id, uuid::Uuid::new_v4()).await
    }
}
