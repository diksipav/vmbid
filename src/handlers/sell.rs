use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use validator::Validate;

use crate::{errors::Result, models::*, state::AppState};

/// Match incoming supply against highest-price bids (FIFO per price level).
/// Update allocations and store leftovers in global supply.
pub async fn handle_sell(
    State(state): State<AppState>,
    Json(payload): Json<SellRequest>,
) -> Result<(StatusCode, Json<SellResponse>)> {
    payload.validate()?;

    let remaining = state.match_sell_with_bids(payload.volume);
    if remaining > 0 {
        state.add_supply(remaining);
    }

    Ok((StatusCode::OK, Json(SellResponse { allocated: payload.volume - remaining })))
}
