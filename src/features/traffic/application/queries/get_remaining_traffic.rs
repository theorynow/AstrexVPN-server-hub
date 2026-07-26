use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::traffic::application::ports::TrafficRepository,
};

pub struct GetRemainingTrafficQuery {
    repo: Arc<dyn TrafficRepository>,
}

impl GetRemainingTrafficQuery {
    pub fn new(repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: &str) -> Result<u64, AppError> {
        self.repo.get_remaining(user_id).await
    }
}
