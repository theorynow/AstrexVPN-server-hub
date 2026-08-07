use axum::{routing::post, Router};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

use super::dto::{PromoCodeDto, UsePromoCodeDto, UsePromoCodeResponseDto};
use super::handlers;
use crate::common::app::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::get_trial_promocode,
        handlers::use_promocode
    ),
    components(schemas(
        PromoCodeDto,
        UsePromoCodeDto,
        UsePromoCodeResponseDto
    )),
    tags((name = "Promocodes", description = "Promo code management endpoints")),
    security(("bearer_auth" = [])),
    modifiers(&PromoCodeApiDoc)
)]
pub struct PromoCodeApiDoc;

impl utoipa::Modify for PromoCodeApiDoc {
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

pub fn promocode_routes() -> Router<AppState> {
    Router::new()
        .route("/trial", post(handlers::get_trial_promocode))
        .route("/use", post(handlers::use_promocode))
}
