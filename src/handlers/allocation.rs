use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
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
    query: std::result::Result<Query<AllocationQuery>, QueryRejection>,
) -> Result<(StatusCode, Json<AllocationResponse>)> {
    let Query(query) = query?;
    query.validate()?;
    let username = query.username;

    let allocated = state.allocation(&username).ok_or(VmbidError::NotFound(username))?;

    Ok((StatusCode::OK, Json(AllocationResponse { allocated })))
}
