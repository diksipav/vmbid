use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use validator::Validate;

use crate::{
    errors::{Result, VmbidError},
    models::*,
    state::AppState,
};

/// Return existing allocation for a user.
pub async fn handle_allocation(
    State(state): State<AppState>,
    Query(query): Query<AllocationQuery>,
) -> Result<(StatusCode, Json<AllocationResponse>)> {
    query.validate()?;
    let username = query.username;

    let allocated = state
        .allocations
        .lock()
        .get(&username)
        .copied()
        .ok_or_else(|| VmbidError::NotFound(username))?;

    Ok((StatusCode::OK, Json(AllocationResponse { allocated })))
}
