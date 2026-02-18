use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, post, web};
use log::info;

/// Matches incoming supply against highest-price bids (FIFO per price level).
/// Updates allocations and stores leftovers in global supply.
pub async fn handle_sell(state: &AppState, supply: u64) {
    let mut remaining = supply;

    if remaining == 0 {
        return;
    }

    let mut bids_guard = state.bids.lock();

    // If there are available bids match them with supply and create allocations.
    if !bids_guard.is_empty() {
        let mut allocations_guard = state.allocations.lock();
        // Match highest-price bids first (BTreeMap::rev())
        for (_price, queue) in bids_guard.iter_mut().rev() {
            let mut front_volume: Option<u64> = None;
            while remaining > 0 && !queue.is_empty() {
                if let Some(mut front) = queue.peek_mut() {
                    let alloc = remaining.min(front.volume);
                    let prior = *allocations_guard.entry(front.username.clone()).or_insert(0);
                    *allocations_guard.get_mut(&front.username).unwrap() += alloc;
                    info!(
                        "User {} allocation: {} -> {}",
                        front.username,
                        prior,
                        prior + alloc
                    );

                    front.volume -= alloc;
                    remaining -= alloc;
                    front_volume = Some(front.volume);
                }
                // Remove if fully filled.
                if front_volume == Some(0) {
                    queue.pop();
                }
            }
            if remaining == 0 {
                break;
            }
        }

        // Clean empty price queues.
        bids_guard.retain(|_, q| !q.is_empty());
    }

    drop(bids_guard);

    // If after all bids are matched there are remaining units, add them to the supply.
    if remaining > 0 {
        let mut supply_guard = state.supply.lock();
        let prior = *supply_guard;
        *supply_guard += remaining;
        info!("Supply increased: {} -> {}", prior, prior + remaining);
    }
}

#[post("/sell")]
pub async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> HttpResponse {
    handle_sell(&state, req.volume).await;

    HttpResponse::Ok().finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web;
    use std::collections::{BTreeMap, BinaryHeap};

    #[actix_web::test]
    async fn test_sell_when_no_bids_creates_supply() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        handle_sell(&data, 100).await;

        assert_eq!(*state.supply.lock(), 100);
    }

    #[actix_web::test]
    async fn test_sell_with_excess_volume() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        // Populate state bids with one element
        let mut bids = BTreeMap::new();
        let mut queue = BinaryHeap::new();
        queue.push(Bid {
            username: "u1".to_string(),
            price: 5,
            volume: 100,
            seq: 0,
        });
        bids.insert(5, queue);
        *state.bids.lock() = bids;

        handle_sell(&data, 200).await;

        assert_eq!(*state.supply.lock(), 100);
        assert!(state.bids.lock().is_empty());
        assert_eq!(*state.allocations.lock().get("u1").unwrap(), 100);
    }
}
