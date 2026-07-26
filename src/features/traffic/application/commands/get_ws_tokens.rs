use crate::common::{
    http::error::AppError,
    security::jwt::{make_centrifugo_connect_token, make_centrifugo_subscribe_token},
};

#[derive(Default)]
pub struct GetWsTokensCommand;

impl GetWsTokensCommand {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, user_id: &str) -> Result<(String, String, String), AppError> {
        let channel = format!("personal:{}", user_id);
        let connection_token = make_centrifugo_connect_token(user_id)?;
        let subscription_token = make_centrifugo_subscribe_token(user_id, &channel)?;
        Ok((connection_token, subscription_token, channel))
    }
}
