use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        traffic::application::ports::TrafficRepository,
        user::{UserProfile, UserRepository},
    },
};

pub struct GetUsersQuery {
    repo: Arc<dyn UserRepository>,
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl GetUsersQuery {
    pub fn new(repo: Arc<dyn UserRepository>, traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo, traffic_repo }
    }

    pub async fn execute(&self) -> Result<Vec<UserProfile>, AppError> {
        let users = self.repo.find_all().await?;
        let mut profiles = Vec::with_capacity(users.len());

        for user in users {
            let summary = self.traffic_repo.get_summary(&user.id).await?;
            profiles.push(UserProfile::resolve(user, summary).await?);
        }

        Ok(profiles)
    }
}
