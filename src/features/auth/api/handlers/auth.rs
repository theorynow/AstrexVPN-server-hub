use crate::{
    common::{
        app::state::AuthState,
        http::{current_user::CurrentUser, dto::RestApiResponse, error::AppError},
        security::jwt::AuthBody,
    },
    features::auth::api::{
        dto::request::{
            AuthUserDto, ChangePasswordDto, RefreshSessionDto, RegisterAuthUserDto,
        },
        handlers::validation::{
            validate_auth_user, validate_change_password,
            validate_refresh_session, validate_register_auth_user,
        },
    },
};
use axum::extract::State;
use axum::{response::IntoResponse, Json};

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterAuthUserDto,
    responses((status = 200, description = "Create user authentication", body = RegisterAuthUserDto)),
    tag = "UserAuth"
)]
pub async fn create_user_auth(
    State(state): State<AuthState>,
    Json(payload): Json<RegisterAuthUserDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_register_auth_user(&payload)?;

    state.register_user.execute(payload.into()).await?;
    Ok(RestApiResponse::success(()))
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
    responses((status = 200, description = "Authenticate as guest", body = AuthBody)),
    tag = "UserAuth"
)]
pub async fn auth_as_guest(
    State(state): State<AuthState>,
) -> Result<impl IntoResponse, AppError> {
    let auth_body = state.auth_as_guest.execute().await?;
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

#[utoipa::path(
    post,
    path = "/auth/change-password",
    request_body = ChangePasswordDto,
    responses((status = 200, description = "Password changed successfully")),
    security(("bearer_auth" = [])),
    tag = "UserAuth"
)]
pub async fn change_password(
    State(state): State<AuthState>,
    current_user: CurrentUser,
    Json(payload): Json<ChangePasswordDto>,
) -> Result<impl IntoResponse, AppError> {
    validate_change_password(&payload)?;

    state
        .change_password
        .execute(current_user.user_id, payload.new_password)
        .await?;
    Ok(RestApiResponse::success(()))
}
