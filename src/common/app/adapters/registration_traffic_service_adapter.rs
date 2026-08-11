use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        auth::RegistrationTrafficService,
        traffic::TrafficRepository,
    },
};

pub struct RegistrationTrafficServiceAdapter {
    traffic_repo: Arc<dyn TrafficRepository>,
}

impl RegistrationTrafficServiceAdapter {
    pub fn new(traffic_repo: Arc<dyn TrafficRepository>) -> Self {
        Self { traffic_repo }
    }
}

#[async_trait]
impl RegistrationTrafficService for RegistrationTrafficServiceAdapter {
    async fn grant_initial_traffic(&self, user_id: &str) -> Result<(), AppError> {
        // 1 GB (1073741824 bytes) for 30 days
        self.traffic_repo
            .add_packet_with_expiry(user_id, 1073741824, 30)
            .await?;
        Ok(())
    }
}
