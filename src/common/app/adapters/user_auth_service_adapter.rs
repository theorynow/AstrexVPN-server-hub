use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        auth::ChangePasswordCommand,
        user::UserAuthService,
    },
};

pub struct UserAuthServiceAdapter {
    change_password_cmd: Arc<ChangePasswordCommand>,
}

impl UserAuthServiceAdapter {
    pub fn new(change_password_cmd: Arc<ChangePasswordCommand>) -> Self {
        Self { change_password_cmd }
    }
}

#[async_trait]
impl UserAuthService for UserAuthServiceAdapter {
    async fn change_password(&self, user_id: &str, new_password: &str) -> Result<(), AppError> {
        self.change_password_cmd
            .execute(user_id.to_string(), new_password.to_string())
            .await
    }
}
