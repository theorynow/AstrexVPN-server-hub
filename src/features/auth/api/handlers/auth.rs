use crate::{
    common::{
        app::state::AuthState,
        http::{dto::RestApiResponse, error::AppError},
        security::jwt::AuthBody,
    },
    features::auth::api::{
        dto::{
            request::{AuthUserDto, GuestAuthDto, RefreshSessionDto, RegisterAuthUserDto},
            response::RegisterResponseDto,
        },
        handlers::validation::{
            validate_auth_user, validate_guest_auth, validate_refresh_session,
            validate_register_auth_user,
        },
    },
};
use axum::extract::State;
use axum::{response::IntoResponse, Json};

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterAuthUserDto,
    responses((status = 200, description = "Create user authentication", body = RegisterResponseDto)),
    tag = "UserAuth"
)]
pub async fn create_user_auth(
    State(state): State<AuthState>,
    Json(payload): Json<RegisterAuthUserDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_register_auth_user(&payload)?;

    let user_id = state.register_user.execute(payload.into()).await?;
    Ok(RestApiResponse::success(RegisterResponseDto { user_id }))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = AuthUserDto,
    responses((status = 200, description = "Login user", body = AuthBody)),
    tag = "UserAuth"
)]
pub async fn login_user(
    State(state): State<AuthState>,
    Json(payload): Json<AuthUserDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_auth_user(&payload)?;

    let auth_body = state.login_user.execute(payload.into()).await?;
    Ok(RestApiResponse::success(auth_body))
}

#[utoipa::path(
    post,
    path = "/auth/guest",
    request_body = GuestAuthDto,
    responses((status = 200, description = "Authenticate as guest", body = AuthBody)),
    tag = "UserAuth"
)]
pub async fn auth_as_guest(
    State(state): State<AuthState>,
    Json(payload): Json<GuestAuthDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_guest_auth(&payload)?;

    let auth_body = state
        .auth_as_guest
        .execute(&payload.device_key, &payload.platform)
        .await?;
    Ok(RestApiResponse::success(auth_body))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshSessionDto,
    responses((status = 200, description = "Refresh session", body = AuthBody)),
    tag = "UserAuth"
)]
pub async fn refresh_session(
    State(state): State<AuthState>,
    Json(payload): Json<RefreshSessionDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_refresh_session(&payload)?;

    let auth_body = state.refresh_session.execute(payload.into()).await?;
    Ok(RestApiResponse::success(auth_body))
}
