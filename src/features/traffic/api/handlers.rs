use axum::{extract::State, Json, response::IntoResponse};

use crate::{
    common::app::state::AppState,
    common::http::{dto::RestApiResponse, error::AppError, current_user::CurrentUser},
    features::traffic::api::dto::{AddTrafficDto, TrafficPacketDto, CentrifugeTokenDto, TrafficSummaryDto},
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
    responses((status = 200, description = "Add traffic packet to user", body = TrafficPacketDto)),
    tag = "Traffic",
    security(("bearer_auth" = []))
)]
pub async fn add_traffic(
    State(state): State<AppState>,
    Json(payload): Json<AddTrafficDto>,
) -> Result<impl IntoResponse, AppError> {
    let bytes = payload.gb * 1024 * 1024 * 1024;
    let packet = state.traffic.add_traffic.execute(&payload.user_id, bytes).await?;
    let dto: TrafficPacketDto = packet.into();
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
    let (connection_token, subscription_token, channel) = state.traffic.get_ws_tokens.execute(&current_user.user_id)?;
    let dto = CentrifugeTokenDto {
        connection_token,
        subscription_token,
        channel,
    };
    Ok(RestApiResponse::success(dto))
}

