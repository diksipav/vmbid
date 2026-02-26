use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use validator::ValidationErrors;

/// Application error types
#[derive(thiserror::Error, Debug)]
pub enum VmbidError {
    #[error("username {0} not found")]
    NotFound(String),
    #[error("{0}")]
    Validation(#[from] ValidationErrors),
}

/// JSON error response body
#[derive(serde::Serialize)]
struct ErrorResponse {
    message: String,
}

impl IntoResponse for VmbidError {
    fn into_response(self) -> Response {
        let status = match &self {
            VmbidError::NotFound(_) => StatusCode::NOT_FOUND,
            VmbidError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };

        let body = ErrorResponse { message: self.to_string() };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for handlers
pub type Result<T> = std::result::Result<T, VmbidError>;
