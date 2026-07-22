use std::sync::Arc;

use crate::features::traffic::{AddTrafficCommand, GetWsTokensCommand, GetTrafficSummaryQuery};

#[derive(Clone)]
pub struct TrafficState {
    pub add_traffic: Arc<AddTrafficCommand>,
    pub get_ws_tokens: Arc<GetWsTokensCommand>,
    pub get_summary: Arc<GetTrafficSummaryQuery>,
}

impl TrafficState {
    pub fn new(
        add_traffic: Arc<AddTrafficCommand>,
        get_ws_tokens: Arc<GetWsTokensCommand>,
        get_summary: Arc<GetTrafficSummaryQuery>,
    ) -> Self {
        Self {
            add_traffic,
            get_ws_tokens,
            get_summary,
        }
    }
}
