use axum::{routing::post, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use super::dto::{
    AddTrafficDto, CentrifugeTokenDto, SetTrafficDto, SubtractTrafficDto, TrafficPacketDto,
    TrafficSummaryDto,
};
use super::handlers;
use crate::common::app::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::add_traffic,
        handlers::subtract_traffic,
        handlers::set_traffic,
        handlers::get_ws_tokens,
        handlers::get_my_traffic
    ),
    components(schemas(
        AddTrafficDto,
        SubtractTrafficDto,
        SetTrafficDto,
        TrafficPacketDto,
        CentrifugeTokenDto,
        TrafficSummaryDto
    )),
    tags((name = "Traffic", description = "Traffic management endpoints")),
    security(("bearer_auth" = [])),
    modifiers(&TrafficApiDoc)
)]
pub struct TrafficApiDoc;

impl utoipa::Modify for TrafficApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your `<your-jwt>`"))
                    .build(),
            ),
        )
    }
}

pub fn traffic_routes() -> Router<AppState> {
    Router::new()
        .route("/add", post(handlers::add_traffic))
        .route("/subtract", post(handlers::subtract_traffic))
        .route("/set", post(handlers::set_traffic))
        .route("/ws-tokens", axum::routing::get(handlers::get_ws_tokens))
        .route("/me", axum::routing::get(handlers::get_my_traffic))
}
