use axum::{extract::State, response::IntoResponse, Json};

use crate::{
    common::app::state::AppState,
    common::http::{current_user::CurrentUser, dto::RestApiResponse, error::AppError},
    features::traffic::api::dto::{
        CentrifugeTokenDto, SetTrafficDto, TrafficHistoryItemDto,
        TrafficHistoryResponseDto, TrafficSummaryDto,
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
    path = "/traffic/set",
    request_body = SetTrafficDto,
    responses((status = 200, description = "Set user remaining traffic to target MB", body = TrafficSummaryDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn set_traffic(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(payload): Json<SetTrafficDto>,
) -> Result<impl IntoResponse, AppError> {
    let target_user_id = match payload.user_id {
        Some(ref id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => current_user.user_id.clone(),
    };
    let target_bytes = payload.mb * 1024 * 1024;
    let summary = state.traffic.set_traffic.execute(&target_user_id, target_bytes).await?;
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

#[utoipa::path(
    get,
    path = "/traffic/history",
    responses((status = 200, description = "Get traffic allocation history for current user", body = TrafficHistoryResponseDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn get_traffic_history(
    State(state): State<AppState>,
    current_user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let packets = state.traffic.get_history.execute(&current_user.user_id).await?;
    let items: Vec<TrafficHistoryItemDto> = packets.into_iter().map(Into::into).collect();
    let dto = TrafficHistoryResponseDto {
        server_time: chrono::Utc::now().to_rfc3339(),
        items,
    };
    Ok(RestApiResponse::success(dto))
}
