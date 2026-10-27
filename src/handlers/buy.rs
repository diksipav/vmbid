use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use validator::Validate;

use crate::{errors::Result, models::*, state::AppState};

/// Allocate volume to the user by matching with existing supply, create bids from remainder.
pub async fn handle_buy(
    State(state): State<AppState>,
    payload: std::result::Result<Json<BuyRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<BuyResponse>)> {
    let Json(payload) = payload?;
    payload.validate()?;
    let BuyRequest { username, volume, price } = payload;

    let allocated = state.buy(username.as_str(), volume, price);

    Ok((StatusCode::OK, Json(BuyResponse { allocated, queued: volume - allocated })))
}
