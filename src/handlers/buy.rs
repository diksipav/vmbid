use axum::{Json, extract::State, http::StatusCode};
use validator::Validate;

use crate::{errors::Result, models::*, state::AppState};

/// Allocate volume to the user by matching with existing supply, create bids from remainder.
pub async fn handle_buy(
    State(state): State<AppState>,
    Json(payload): Json<BuyRequest>,
) -> Result<(StatusCode, Json<BuyResponse>)> {
    payload.validate()?;
    let BuyRequest { username, volume, price } = payload;

    let allocated = state.consume_supply_and_allocate(username.as_str(), volume);
    let remaining = volume - allocated;
    if remaining > 0 {
        state.queue_bid(username.as_str(), remaining, price);
    }

    Ok((StatusCode::OK, Json(BuyResponse { allocated, queued: remaining })))
}
