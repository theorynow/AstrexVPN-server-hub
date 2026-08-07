use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    common::app::state::AppState,
    common::http::{
        current_user::{CurrentUser, OptionalCurrentUser},
        dto::RestApiResponse,
        error::AppError,
    },
    features::promocode::api::dto::{PromoCodeDto, UsePromoCodeDto, UsePromoCodeResponseDto},
};

#[utoipa::path(
    post,
    path = "/promocodes/trial",
    responses((status = 200, description = "Get or create trial promo code", body = PromoCodeDto)),
    tag = "Promocodes"
)]
pub async fn get_trial_promocode(
    State(state): State<AppState>,
    OptionalCurrentUser(current_user): OptionalCurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let user_id = current_user.map(|u| u.user_id);
    let promocode = state
        .promocode
        .get_or_create_trial
        .execute(user_id.as_deref())
        .await?;
    let dto: PromoCodeDto = promocode.into();
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    post,
    path = "/promocodes/use",
    request_body = UsePromoCodeDto,
    responses((status = 200, description = "Redeem a 5-character promo code", body = UsePromoCodeResponseDto)),
    tag = "Promocodes",
    security(("bearer_auth" = []))
)]
pub async fn use_promocode(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(payload): Json<UsePromoCodeDto>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .promocode
        .use_promocode
        .execute(&current_user.user_id, &payload.code)
        .await?;

    let response_dto = UsePromoCodeResponseDto {
        reward_type: result.reward_type.to_string(),
        reward_bytes: result.reward_bytes,
        duration_days: result.duration_days,
    };
    Ok(RestApiResponse::success(response_dto))
}
