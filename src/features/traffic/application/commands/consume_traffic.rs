use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::traffic::application::ports::TrafficRepository,
};

pub struct ConsumeTrafficCommand {
    repo: Arc<dyn TrafficRepository>,
}

impl ConsumeTrafficCommand {
    pub fn new(repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: &str, bytes: u64) -> Result<u64, AppError> {
        self.repo.consume(user_id, bytes).await
    }
}
