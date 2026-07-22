use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::traffic::{application::ports::TrafficRepository, domain::model::TrafficSummary},
};

pub struct GetTrafficSummaryQuery {
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl GetTrafficSummaryQuery {
    pub fn new(traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { traffic_repo }
    }

    pub async fn execute(&self, user_id: &str) -> Result<TrafficSummary, AppError> {
        self.traffic_repo.get_summary(user_id).await
    }
}
