use axum::{
    extract::{Json, State, rejection::JsonRejection},
    http::StatusCode,
};
use validator::Validate;

use crate::{errors::Result, models::*, state::AppState};

/// Match incoming supply against highest-price bids (FIFO per price level).
/// Update allocations and store leftovers in global supply.
pub async fn handle_sell(
    State(state): State<AppState>,
    payload: std::result::Result<Json<SellRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SellResponse>)> {
    let Json(payload) = payload?;
    payload.validate()?;

    let allocated = state.sell(payload.volume)?;

    Ok((StatusCode::OK, Json(SellResponse { allocated })))
}
