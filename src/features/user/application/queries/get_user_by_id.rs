use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        traffic::application::ports::TrafficRepository,
        user::{UserProfile, UserRepository},
    },
};

pub struct GetUserByIdQuery {
    repo: Arc<dyn UserRepository>,
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl GetUserByIdQuery {
    pub fn new(repo: Arc<dyn UserRepository>, traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo, traffic_repo }
    }

    pub async fn execute(&self, id: String) -> Result<UserProfile, AppError> {
        let user = self
            .repo
            .find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        let summary = self.traffic_repo.get_summary(&id).await?;

        UserProfile::resolve(user, summary).await
    }
}
