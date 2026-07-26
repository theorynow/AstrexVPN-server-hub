use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::{
        nodes::application::ports::UserTrafficService,
        traffic::{
            application::commands::consume_traffic::ConsumeTrafficCommand,
            application::queries::get_remaining_traffic::GetRemainingTrafficQuery,
        },
    },
};

pub struct UserTrafficServiceAdapter {
    consume_traffic_cmd: Arc<ConsumeTrafficCommand>,
    get_remaining_traffic_query: Arc<GetRemainingTrafficQuery>,
}

impl UserTrafficServiceAdapter {
    pub fn new(
        consume_traffic_cmd: Arc<ConsumeTrafficCommand>,
        get_remaining_traffic_query: Arc<GetRemainingTrafficQuery>,
    ) -> Self {
        Self {
            consume_traffic_cmd,
            get_remaining_traffic_query,
        }
    }
}

#[async_trait]
impl UserTrafficService for UserTrafficServiceAdapter {
    async fn get_remaining_traffic(&self, user_uuid: &str) -> Result<u64, AppError> {
        self.get_remaining_traffic_query.execute(user_uuid).await
    }

    async fn consume_traffic(&self, user_uuid: &str, bytes: u64) -> Result<u64, AppError> {
        self.consume_traffic_cmd.execute(user_uuid, bytes).await
    }
}
