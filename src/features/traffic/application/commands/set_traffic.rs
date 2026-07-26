use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::traffic::{
        application::ports::TrafficRepository,
        domain::model::TrafficSummary,
    },
};

/// Sets the user's remaining traffic to `target_bytes`.
pub struct SetTrafficCommand {
    repo: Arc<dyn TrafficRepository>,
}

impl SetTrafficCommand {
    pub fn new(repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: &str, target_bytes: u64) -> Result<TrafficSummary, AppError> {
        let current_remaining = self.repo.get_remaining(user_id).await?;
        if target_bytes > current_remaining {
            let diff = (target_bytes - current_remaining) as i64;
            self.repo.add_packet(user_id, diff).await?;
        } else if target_bytes < current_remaining {
            let diff = current_remaining - target_bytes;
            self.repo.consume(user_id, diff).await?;
        }
        self.repo.get_summary(user_id).await
    }
}
