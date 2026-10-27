use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
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
    #[error("supply limit exceeded")]
    SupplyOverflow,
    /// Body extraction failed (malformed JSON, missing field, wrong content type).
    #[error(transparent)]
    Json(#[from] JsonRejection),
    /// Query string extraction failed (e.g. missing username).
    #[error(transparent)]
    Query(#[from] QueryRejection),
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
            VmbidError::Validation(_) | VmbidError::SupplyOverflow => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            // keep axum's status codes (400/415/422), only the body becomes JSON
            VmbidError::Json(e) => e.status(),
            VmbidError::Query(e) => e.status(),
        };

        let body = ErrorResponse { message: self.to_string() };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for handlers
pub type Result<T> = std::result::Result<T, VmbidError>;
