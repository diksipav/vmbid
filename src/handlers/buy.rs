use crate::errors::VmbidError;
use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, post, web};
use log::{error, info};
use std::collections::BinaryHeap;
use std::sync::atomic::Ordering;

/// Handles buy requests: allocates from supply, creates bids from remainder.
pub async fn handle_buy(
    state: &AppState,
    username: String,
    volume: u64,
    price: u64,
) -> Result<(), VmbidError> {
    if username.trim().is_empty() {
        error!(
            "Buy rejected: missing username for volume={} price={}",
            volume, price
        );
        return Err(VmbidError::MissingUsername);
    }

    let mut remaining = volume;
    if remaining == 0 {
        return Ok(());
    }

    // Compares requested volume to available supply and calculates the amount to allocate.
    // Updates supply and remaining volume.
    {
        let mut supply_guard = state.supply.lock();
        let alloc = remaining.min(*supply_guard);
        *supply_guard -= alloc;
        remaining -= alloc;
    }

    // Creates allocation for the user.
    if remaining < volume {
        let mut allocations_guard = state.allocations.lock();
        let prior = *allocations_guard.entry(username.clone()).or_insert(0);
        *allocations_guard.get_mut(&username).unwrap() += volume - remaining;
        info!(
            "User {} allocation: {} -> {}",
            username,
            prior,
            prior + volume - remaining
        );
    }

    // Queue remainder as bid.
    if remaining > 0 {
        let seq = state.seq.fetch_add(1, Ordering::Relaxed);
        let bid = Bid {
            username: username.clone(),
            volume: remaining,
            price,
            seq,
        };

        let mut bids_guard = state.bids.lock();
        bids_guard
            .entry(price)
            .or_insert_with(BinaryHeap::new)
            .push(bid);
        info!(
            "Queued bid: user={} volume={} price={} seq={}",
            username, remaining, price, seq
        );
    }

    Ok(())
}

#[post("/buy")]
pub async fn buy(
    state: web::Data<AppState>,
    req: web::Json<BuyRequest>,
) -> Result<HttpResponse, VmbidError> {
    let BuyRequest {
        username,
        volume,
        price,
    } = req.into_inner();

    handle_buy(&state, username, volume, price).await?;

    Ok(HttpResponse::Ok().finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web;

    #[actix_web::test]
    async fn test_buy_with_no_supply_creates_bid() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 100, 3).await;
        assert!(result.is_ok());

        let bids = state.bids.lock();
        let heap = bids.get(&3).unwrap();

        assert_eq!(heap.len(), 1);
        assert_eq!(
            heap.peek(),
            Some(&Bid {
                username: "u1".to_string(),
                volume: 100,
                price: 3,
                seq: 0,
            })
        );
    }

    #[actix_web::test]
    async fn test_buy_with_supply_immediate_allocation() {
        let state = AppState::default();
        // Set initial supply
        *state.supply.lock() = 150;

        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 100, 3).await;
        assert!(result.is_ok());

        // Supply is updated. No bids.
        assert_eq!(state.bids.lock().len(), 0);
        assert_eq!(*state.supply.lock(), 50);

        // Allocation is created
        let allocations = state.allocations.lock();
        assert_eq!(allocations.len(), 1);
        let allocation = allocations.get("u1").unwrap();
        assert_eq!(*allocation, 100);
    }

    #[actix_web::test]
    async fn test_buy_with_partial_supply() {
        let state = AppState::default();
        // Set initial supply
        *state.supply.lock() = 50;

        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 200, 4).await;
        assert!(result.is_ok());

        // Required resources are higner than supply,
        // so both allocation and bid are created.
        // Supply is emptied.
        let bids = state.bids.lock();
        let heap = bids.get(&4).unwrap();
        assert_eq!(heap.len(), 1);
        assert_eq!(
            heap.peek(),
            Some(&Bid {
                username: "u1".to_string(),
                volume: 150,
                price: 4,
                seq: 0,
            })
        );

        assert_eq!(*state.supply.lock(), 0);

        let allocations = state.allocations.lock();
        assert_eq!(allocations.len(), 1);
        let allocation = allocations.get("u1").unwrap();
        assert_eq!(*allocation, 50);
    }

    #[actix_web::test]
    async fn test_buy_with_0_volume() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 0, 3).await;
        assert!(result.is_ok());

        // Could also test if all locks have default values.
    }

    #[actix_web::test]
    #[should_panic(expected = "MissingUsername")]
    async fn test_buy_with_empty_username() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        handle_buy(&data, "".to_string(), 100, 3).await.unwrap();
    }

    #[actix_web::test]
    async fn test_concurrent_buys_fifo_ordering() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        // Spawn two concurrent buys
        let handle1 = {
            let data = data.clone();
            tokio::spawn(async move { handle_buy(&data, "u1".to_string(), 100, 5).await })
        };

        let handle2 = {
            let data = data.clone();
            tokio::spawn(async move { handle_buy(&data, "u2".to_string(), 50, 5).await })
        };

        // Wait for both to complete
        handle1.await.unwrap().unwrap();
        handle2.await.unwrap().unwrap();
        // assert!(handle1.await.unwrap().is_ok());
        // assert!(handle2.await.unwrap().is_ok());

        // Assert that there is a queue created for
        // the price 5 with sequence increasing.
        let bids = state.bids.lock();
        let heap = bids.get(&5).unwrap();
        assert_eq!(heap.len(), 2);

        let heap_vec: Vec<&Bid> = heap.iter().collect();

        assert_eq!(
            heap_vec[0],
            &Bid {
                username: "u1".to_string(),
                volume: 100,
                price: 5,
                seq: 0,
            }
        );

        assert_eq!(
            heap_vec[1],
            &Bid {
                username: "u2".to_string(),
                volume: 50,
                price: 5,
                seq: 1,
            }
        );
    }
}
