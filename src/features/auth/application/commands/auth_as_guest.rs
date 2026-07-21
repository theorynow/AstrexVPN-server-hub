use std::sync::Arc;

use uuid::Uuid;

use crate::{
    common::{http::error::AppError, security::jwt::AuthBody},
    features::auth::{
        application::commands::session_tokens::issue_tokens_with_family, AuthRepository,
    },
};

pub struct AuthAsGuestCommand {
    repo: Arc<dyn AuthRepository>,
}

impl AuthAsGuestCommand {
    pub fn new(repo: Arc<dyn AuthRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self) -> Result<AuthBody, AppError> {
        // We pre-generate an id so we can use it in the username,
        // but the repo generates its own id — so we use the returned one for the token.
        let tentative_id = Uuid::new_v4();
        let username = format!("guest-{}", tentative_id);

        let user_id = self
            .repo
            .create_user_with_auth(Some(username), None)
            .await?;

        issue_tokens_with_family(&self.repo, user_id, Uuid::new_v4()).await
    }
}
