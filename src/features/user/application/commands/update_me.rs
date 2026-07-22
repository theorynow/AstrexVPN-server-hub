use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        traffic::application::ports::TrafficRepository,
        user::{UpdateUserProfile, UserProfile, UserRepository},
    },
};

pub struct UpdateMeCommand {
    repo: Arc<dyn UserRepository>,
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl UpdateMeCommand {
    pub fn new(repo: Arc<dyn UserRepository>, traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo, traffic_repo }
    }

    pub async fn execute(
        &self,
        user_id: String,
        update: UpdateUserProfile,
    ) -> Result<UserProfile, AppError> {
        let update = normalize_update(update)?;
        let user = self
            .repo
            .update_profile(&user_id, update)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        let summary = self.traffic_repo.get_summary(&user_id).await?;

        UserProfile::resolve(user, summary).await
    }
}

fn normalize_update(update: UpdateUserProfile) -> Result<UpdateUserProfile, AppError> {
    let username = normalize_optional_field(update.username);

    if username.is_none() {
        return Err(AppError::ValidationError(
            "Username must be provided".into(),
        ));
    }

    Ok(UpdateUserProfile { username })
}

fn normalize_optional_field(field: Option<String>) -> Option<String> {
    field
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
}
