use axum::{
    Json,
    extract::{Query, State},
};

use crate::{errors::VmbidError, models::*, state::AppState};

/// Returns allocation for a given username.
pub async fn handle_allocation(
    state: &AppState,
    username: Option<&str>,
) -> Result<u64, VmbidError> {
    let username = username.ok_or(VmbidError::MissingUsername)?.trim();
    if username.is_empty() {
        return Err(VmbidError::MissingUsername);
    }

    let allocations_guard = state.allocations.lock();
    allocations_guard
        .get(username)
        .copied()
        .ok_or_else(|| VmbidError::NotFound(username.to_string()))
}

pub async fn allocation(
    State(state): State<AppState>,
    Query(query): Query<AllocationQuery>,
) -> Result<Json<u64>, VmbidError> {
    let allocation = handle_allocation(&state, query.username.as_deref()).await?;
    Ok(Json(allocation))
}
