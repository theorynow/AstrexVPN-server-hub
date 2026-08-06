use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    BoxError,
};

use sqlx::Error as SqlxError;
use thiserror::Error;
use tracing::error;

use crate::common::http::dto::RestApiResponse;

use super::dto::ApiResponse;

/// AppError is an enum that represents various types of errors that can occur in the application.
/// It implements the `std::error::Error` trait and the `axum::response::IntoResponse` trait.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] SqlxError), // Used for database-related errors

    #[error("Not found: {0}")]
    NotFound(String), // Used for not found errors

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    InternalError,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Forbidden Request: {0}")]
    Forbidden(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Used for authentication-related errors
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token creation error")]
    TokenCreation,
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Node rejected action")]
    NodeRejectedAction,

    #[error("User has no remaining traffic")]
    TrafficExhausted,
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::DatabaseError(_) => "DATABASE_ERROR",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::InternalError => "INTERNAL_SERVER_ERROR",
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::ProviderError(_) => "PROVIDER_ERROR",
            AppError::WrongCredentials => "WRONG_CREDENTIALS",
            AppError::MissingCredentials => "MISSING_CREDENTIALS",
            AppError::InvalidToken => "INVALID_TOKEN",
            AppError::TokenCreation => "TOKEN_CREATION_ERROR",
            AppError::UserNotFound => "USER_NOT_FOUND",
            AppError::UserAlreadyExists => "USER_ALREADY_EXISTS",
            AppError::NodeRejectedAction => "NODE_REJECTED_ACTION",
            AppError::TrafficExhausted => "TRAFFIC_EXHAUSTED",
        }
    }
}

/// Converts the AppError enum into an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::ProviderError(_) => StatusCode::BAD_GATEWAY,
            AppError::WrongCredentials => StatusCode::UNAUTHORIZED,
            AppError::MissingCredentials => StatusCode::BAD_REQUEST,
            AppError::InvalidToken => StatusCode::UNAUTHORIZED,
            AppError::TokenCreation => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::UserNotFound => StatusCode::NOT_FOUND,
            AppError::UserAlreadyExists => StatusCode::CONFLICT,
            AppError::NodeRejectedAction => StatusCode::BAD_REQUEST,
            AppError::TrafficExhausted => StatusCode::FORBIDDEN,
        };

        let message = self.error_code().to_string();

        if status.is_server_error() {
            error!(error = %self, status = status.as_u16(), "Application error");
        } else {
            tracing::warn!(error = %self, status = status.as_u16(), "Client error");
        }

        let body = axum::Json(ApiResponse::<()> {
            status: status.as_u16(),
            message,
            data: None,
            request_id: crate::common::http::dto::get_current_request_id(),
        });

        (status, body).into_response()
    }
}

/// handle_error is a function that middlewares the error handling in the application.
/// It takes a BoxError as input and returns an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
/// The function is used to handle errors that occur during the request processing.
/// It is designed to be used with the axum framework.
pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    let status = if error.is::<tower::timeout::error::Elapsed>() {
        StatusCode::REQUEST_TIMEOUT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    let message = if status == StatusCode::REQUEST_TIMEOUT {
        "REQUEST_TIMEOUT".to_string()
    } else {
        "INTERNAL_SERVER_ERROR".to_string()
    };
    error!(?status, %error, "Request failed");

    let body = RestApiResponse::<()>::failure(status.as_u16(), message);

    (status, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_codes() {
        assert_eq!(AppError::NotFound("test".to_string()).error_code(), "NOT_FOUND");
        assert_eq!(AppError::Conflict("test".to_string()).error_code(), "CONFLICT");
        assert_eq!(AppError::InternalError.error_code(), "INTERNAL_SERVER_ERROR");
        assert_eq!(AppError::ValidationError("test".to_string()).error_code(), "VALIDATION_ERROR");
        assert_eq!(AppError::Forbidden("test".to_string()).error_code(), "FORBIDDEN");
        assert_eq!(AppError::ProviderError("test".to_string()).error_code(), "PROVIDER_ERROR");
        assert_eq!(AppError::WrongCredentials.error_code(), "WRONG_CREDENTIALS");
        assert_eq!(AppError::MissingCredentials.error_code(), "MISSING_CREDENTIALS");
        assert_eq!(AppError::InvalidToken.error_code(), "INVALID_TOKEN");
        assert_eq!(AppError::TokenCreation.error_code(), "TOKEN_CREATION_ERROR");
        assert_eq!(AppError::UserNotFound.error_code(), "USER_NOT_FOUND");
        assert_eq!(AppError::UserAlreadyExists.error_code(), "USER_ALREADY_EXISTS");
        assert_eq!(AppError::NodeRejectedAction.error_code(), "NODE_REJECTED_ACTION");
        assert_eq!(AppError::TrafficExhausted.error_code(), "TRAFFIC_EXHAUSTED");
    }
}
