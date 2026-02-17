use crate::errors::VmbidError;
use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, get, web};

pub async fn handle_allocation(
    state: &AppState,
    username: Option<&str>,
) -> Result<u64, VmbidError> {
    let username = username.ok_or(VmbidError::MissingUsername)?.trim();

    let allocations_guard = state.allocations.lock();
    allocations_guard
        .get(username)
        .copied()
        .ok_or_else(|| VmbidError::NotFound(username.to_string()))
}

#[get("/allocation")]
pub async fn allocation(
    state: web::Data<AppState>,
    query: web::Query<AllocationQuery>,
) -> Result<HttpResponse, VmbidError> {
    let res = handle_allocation(&state, query.username.as_deref()).await?;

    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(res.to_string()))
}
