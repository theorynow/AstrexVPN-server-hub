use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::traffic::{
        application::ports::TrafficRepository,
        domain::model::TrafficSummary,
    },
};

/// Subtracts/deducts `bytes` from the given user's active traffic packets.
pub struct SubtractTrafficCommand {
    repo: Arc<dyn TrafficRepository>,
}

impl SubtractTrafficCommand {
    pub fn new(repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: &str, bytes: u64) -> Result<TrafficSummary, AppError> {
        if bytes == 0 {
            return Err(AppError::ValidationError(
                "Traffic bytes to subtract must be greater than zero".into(),
            ));
        }
        self.repo.consume(user_id, bytes).await?;
        self.repo.get_summary(user_id).await
    }
}
