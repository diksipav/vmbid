use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VmbidError {
    #[error("please provide a username")]
    MissingUsername,

    #[error("username {0} not found")]
    NotFound(String),
}

impl ResponseError for VmbidError {
    fn error_response(&self) -> HttpResponse {
        match self {
            VmbidError::MissingUsername => HttpResponse::BadRequest().body(self.to_string()),
            VmbidError::NotFound(_) => HttpResponse::NotFound().body(self.to_string()),
        }
    }
}
