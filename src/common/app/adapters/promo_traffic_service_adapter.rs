use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        promocode::application::ports::PromoTrafficService,
        traffic::application::commands::add_traffic::AddTrafficCommand,
    },
};

pub struct PromoTrafficServiceAdapter {
    add_traffic_cmd: Arc<AddTrafficCommand>,
}

impl PromoTrafficServiceAdapter {
    pub fn new(add_traffic_cmd: Arc<AddTrafficCommand>) -> Self {
        Self { add_traffic_cmd }
    }
}

#[async_trait]
impl PromoTrafficService for PromoTrafficServiceAdapter {
    async fn grant_traffic(
        &self,
        user_id: &str,
        bytes: i64,
        duration_days: i64,
    ) -> Result<(), AppError> {
        self.add_traffic_cmd
            .execute_with_expiry(user_id, bytes, duration_days)
            .await?;
        Ok(())
    }
}
