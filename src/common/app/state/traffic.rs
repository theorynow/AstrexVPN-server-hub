use std::sync::Arc;

use crate::features::traffic::{
    AddTrafficCommand, GetTrafficHistoryQuery, GetTrafficSummaryQuery, GetWsTokensCommand,
    SetTrafficCommand, SubtractTrafficCommand,
};

#[derive(Clone)]
pub struct TrafficState {
    pub add_traffic: Arc<AddTrafficCommand>,
    pub subtract_traffic: Arc<SubtractTrafficCommand>,
    pub set_traffic: Arc<SetTrafficCommand>,
    pub get_ws_tokens: Arc<GetWsTokensCommand>,
    pub get_summary: Arc<GetTrafficSummaryQuery>,
    pub get_history: Arc<GetTrafficHistoryQuery>,
}

impl TrafficState {
    pub fn new(
        add_traffic: Arc<AddTrafficCommand>,
        subtract_traffic: Arc<SubtractTrafficCommand>,
        set_traffic: Arc<SetTrafficCommand>,
        get_ws_tokens: Arc<GetWsTokensCommand>,
        get_summary: Arc<GetTrafficSummaryQuery>,
        get_history: Arc<GetTrafficHistoryQuery>,
    ) -> Self {
        Self {
            add_traffic,
            subtract_traffic,
            set_traffic,
            get_ws_tokens,
            get_summary,
            get_history,
        }
    }
}
