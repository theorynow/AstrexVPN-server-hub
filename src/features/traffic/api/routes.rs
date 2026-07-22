use axum::{routing::post, Router};
use utoipa::OpenApi;

use crate::common::app::state::AppState;
use super::handlers;
use super::dto::{AddTrafficDto, TrafficPacketDto, CentrifugeTokenDto};

#[derive(OpenApi)]
#[openapi(
    paths(handlers::add_traffic, handlers::get_ws_tokens),
    components(schemas(AddTrafficDto, TrafficPacketDto, CentrifugeTokenDto)),
    tags((name = "Traffic", description = "Traffic management endpoints")),
    security(("bearer_auth" = []))
)]
pub struct TrafficApiDoc;

pub fn traffic_routes() -> Router<AppState> {
    Router::new()
        .route("/add", post(handlers::add_traffic))
        .route("/ws-tokens", axum::routing::get(handlers::get_ws_tokens))
}
