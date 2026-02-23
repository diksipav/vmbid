use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(thiserror::Error, Debug)]
pub enum VmbidError {
    #[error("please provide username")]
    MissingUsername,
    #[error("username {0} not found")]
    NotFound(String),
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    message: String,
}

impl IntoResponse for VmbidError {
    fn into_response(self) -> Response {
        let status = match &self {
            VmbidError::MissingUsername => StatusCode::BAD_REQUEST,
            VmbidError::NotFound(_) => StatusCode::NOT_FOUND,
        };
        (
            status,
            Json(ErrorResponse {
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}
