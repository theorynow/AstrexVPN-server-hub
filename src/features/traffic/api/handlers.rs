use axum::{extract::State, Json, response::IntoResponse};

use crate::{
    common::app::state::AppState,
    common::http::{dto::RestApiResponse, error::AppError, current_user::CurrentUser},
    features::traffic::api::dto::{AddTrafficDto, TrafficPacketDto, CentrifugeTokenDto},
};

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

