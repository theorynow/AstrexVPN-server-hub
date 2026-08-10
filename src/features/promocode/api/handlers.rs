use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::{
    common::app::state::AppState,
    common::http::{
        current_user::CurrentUser,
        dto::RestApiResponse,
        error::AppError,
    },
    features::promocode::api::dto::{PromoCodeInfoDto, UsePromoCodeDto, UsePromoCodeResponseDto},
};

#[utoipa::path(
    get,
    path = "/promocodes/info/{code}",
    responses(
        (status = 200, description = "Get promo code information including remaining uses", body = PromoCodeInfoDto),
        (status = 404, description = "Promo code not found")
    ),
    tag = "Promocodes"
)]
pub async fn get_promocode_info(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let promocode = state.promocode.get_info.execute(&code).await?;
    let dto: PromoCodeInfoDto = promocode.into();
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    post,
    path = "/promocodes/use",
    request_body = UsePromoCodeDto,
    responses((status = 200, description = "Redeem a promo code", body = UsePromoCodeResponseDto)),
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
