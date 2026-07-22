use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        traffic::application::ports::TrafficRepository,
        user::{SearchUser, UserProfile, UserRepository},
    },
};

pub struct GetUserListQuery {
    repo: Arc<dyn UserRepository>,
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl GetUserListQuery {
    pub fn new(repo: Arc<dyn UserRepository>, traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo, traffic_repo }
    }

    pub async fn execute(&self, search: SearchUser) -> Result<Vec<UserProfile>, AppError> {
        let users = self.repo.find_list(search).await?;
        let mut profiles = Vec::with_capacity(users.len());

        for user in users {
            let summary = self.traffic_repo.get_summary(&user.id).await?;
            profiles.push(UserProfile::resolve(user, summary).await?);
        }

        Ok(profiles)
    }
}
