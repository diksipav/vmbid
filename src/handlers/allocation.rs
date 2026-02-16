use crate::errors::VMbidError;
use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, get, web};

pub async fn handle_allocation(
    state: &AppState,
    username: Option<&str>,
) -> Result<u64, VMbidError> {
    let username = username
        .filter(|s| !s.is_empty())
        .ok_or(VMbidError::MissingUsername)?;

    let allocations_guard = state.allocations.lock().unwrap();

    match allocations_guard.get(username) {
        Some(&alloc) => Ok(alloc),
        None => Err(VMbidError::NotFound(username.to_string())),
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
        Err(VMbidError::MissingUsername) => {
            HttpResponse::BadRequest().body("missing 'username' query parameter")
        }
        Err(VMbidError::NotFound(username)) => {
            HttpResponse::NotFound().body(format!("username '{}' not found", username))
        }
    }
}
