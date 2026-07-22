use std::sync::Arc;

use crate::features::traffic::{AddTrafficCommand, GetWsTokensCommand};

#[derive(Clone)]
pub struct TrafficState {
    pub add_traffic: Arc<AddTrafficCommand>,
    pub get_ws_tokens: Arc<GetWsTokensCommand>,
}

impl TrafficState {
    pub fn new(
        add_traffic: Arc<AddTrafficCommand>,
        get_ws_tokens: Arc<GetWsTokensCommand>,
    ) -> Self {
        Self {
            add_traffic,
            get_ws_tokens,
        }
    }
}
