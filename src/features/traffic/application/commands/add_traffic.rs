use std::sync::Arc;
use crate::{
    common::http::error::AppError,
    features::traffic::{
        application::ports::TrafficRepository,
        domain::model::TrafficPacket,
    },
};

/// Adds a new traffic packet of `bytes` size for the given user, valid for 30 days.
pub struct AddTrafficCommand {
    repo: Arc<dyn TrafficRepository>,
}

impl AddTrafficCommand {
    pub fn new(repo: Arc<dyn TrafficRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: &str, bytes: i64) -> Result<TrafficPacket, AppError> {
        if bytes <= 0 {
            return Err(AppError::ValidationError(
                "Traffic bytes must be greater than zero".into(),
            ));
        }
        self.repo.add_packet(user_id, bytes).await
    }
}
