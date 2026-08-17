use std::sync::Arc;

use crate::{
    common::http::error::AppError,
    features::nodes::domain::{
        model::{HysteriaConfig, XrayConfig},
        ports::node_repository::NodeRepository,
    },
};

pub struct ConnectNodeCommand {
    repo: Arc<dyn NodeRepository>,
    expected_secret: String,
}

impl ConnectNodeCommand {
    pub fn new(repo: Arc<dyn NodeRepository>, expected_secret: String) -> Self {
        Self {
            repo,
            expected_secret,
        }
    }

    pub async fn execute(
        &self,
        node_id: &str,
        auth_secret: &str,
        public_ip: &str,
        name_en: &str,
        country_code: &str,
        country_flag: &str,
        xray: Option<XrayConfig>,
        hysteria: Option<HysteriaConfig>,
    ) -> Result<(), AppError> {
        if self.expected_secret != auth_secret {
            return Err(AppError::WrongCredentials);
        }

        self.repo
            .upsert_on_connect(
                node_id,
                public_ip,
                name_en,
                country_code,
                country_flag,
                xray.as_ref(),
                hysteria.as_ref(),
            )
            .await?;

        Ok(())
    }
}
