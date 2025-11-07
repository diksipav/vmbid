use crate::errors::TwinError;
use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, get, web};

pub async fn handle_allocation(state: &AppState, username: Option<&str>) -> Result<u64, TwinError> {
    let username = username
        .filter(|s| !s.is_empty())
        .ok_or(TwinError::MissingUsername)?;

    let allocations_guard = state.state.allocations.lock().unwrap();

    match allocations_guard.get(username) {
        Some(&alloc) => Ok(alloc),
        None => Err(TwinError::NotFound(username.to_string())),
    }
}

#[get("/allocation")]
pub async fn allocation(
    state: web::Data<AppState>,
    query: web::Query<AllocationQuery>,
) -> impl Responder {
    match handle_allocation(&state, query.username.as_deref()).await {
        Ok(allocation) => HttpResponse::Ok()
            .content_type("text/plain")
            .body(allocation.to_string()),
        Err(TwinError::MissingUsername) => {
            HttpResponse::BadRequest().body("missing 'username' query parameter")
        }
        Err(TwinError::NotFound(username)) => {
            HttpResponse::NotFound().body(format!("username '{}' not found", username))
        }
    }
}
