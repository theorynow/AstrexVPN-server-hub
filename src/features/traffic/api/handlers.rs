use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    common::app::state::AppState,
    common::http::{current_user::CurrentUser, dto::RestApiResponse, error::AppError},
    features::traffic::api::dto::{
        AddTrafficDto, CentrifugeTokenDto, SetTrafficDto, SubtractTrafficDto, TrafficPacketDto,
        TrafficSummaryDto,
    },
};

#[utoipa::path(
    get,
    path = "/traffic/me",
    responses((status = 200, description = "Get current user traffic stats", body = TrafficSummaryDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn get_my_traffic(
    State(state): State<AppState>,
    current_user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let summary = state.traffic.get_summary.execute(&current_user.user_id).await?;
    let dto = TrafficSummaryDto {
        traffic_total_bytes: summary.total_bytes,
        traffic_remaining_bytes: summary.remaining_bytes,
        updated_at_ms: summary.updated_at_ms,
    };
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    post,
    path = "/traffic/add",
    request_body = AddTrafficDto,
    responses((status = 200, description = "Add traffic packet in MB to user", body = TrafficPacketDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn add_traffic(
    State(state): State<AppState>,
    Json(payload): Json<AddTrafficDto>,
) -> Result<impl IntoResponse, AppError> {
    if payload.mb <= 0 {
        return Err(AppError::ValidationError(
            "Traffic MB must be greater than zero".into(),
        ));
    }
    let bytes = payload.mb * 1024 * 1024;
    let packet = state.traffic.add_traffic.execute(&payload.user_id, bytes).await?;
    let dto: TrafficPacketDto = packet.into();
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    post,
    path = "/traffic/subtract",
    request_body = SubtractTrafficDto,
    responses((status = 200, description = "Subtract traffic in MB from user", body = TrafficSummaryDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn subtract_traffic(
    State(state): State<AppState>,
    Json(payload): Json<SubtractTrafficDto>,
) -> Result<impl IntoResponse, AppError> {
    if payload.mb == 0 {
        return Err(AppError::ValidationError(
            "Traffic MB to subtract must be greater than zero".into(),
        ));
    }
    let bytes = payload.mb * 1024 * 1024;
    let summary = state.traffic.subtract_traffic.execute(&payload.user_id, bytes).await?;
    let dto = TrafficSummaryDto {
        traffic_total_bytes: summary.total_bytes,
        traffic_remaining_bytes: summary.remaining_bytes,
        updated_at_ms: summary.updated_at_ms,
    };
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    post,
    path = "/traffic/set",
    request_body = SetTrafficDto,
    responses((status = 200, description = "Set user remaining traffic to target MB", body = TrafficSummaryDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn set_traffic(
    State(state): State<AppState>,
    Json(payload): Json<SetTrafficDto>,
) -> Result<impl IntoResponse, AppError> {
    let target_bytes = payload.mb * 1024 * 1024;
    let summary = state.traffic.set_traffic.execute(&payload.user_id, target_bytes).await?;
    let dto = TrafficSummaryDto {
        traffic_total_bytes: summary.total_bytes,
        traffic_remaining_bytes: summary.remaining_bytes,
        updated_at_ms: summary.updated_at_ms,
    };
    Ok(RestApiResponse::success(dto))
}

#[utoipa::path(
    get,
    path = "/traffic/ws-tokens",
    responses((status = 200, description = "Get Centrifugo WebSocket connection and subscription tokens", body = CentrifugeTokenDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn get_ws_tokens(
    State(state): State<AppState>,
    current_user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let (connection_token, subscription_token, channel) = state
        .traffic
        .get_ws_tokens
        .execute(&current_user.user_id)?;
    let dto = CentrifugeTokenDto {
        connection_token,
        subscription_token,
        channel,
    };
    Ok(RestApiResponse::success(dto))
}
