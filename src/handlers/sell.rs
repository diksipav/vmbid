use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, post, web};

pub async fn handle_sell(state: &AppState, supply: u64) {
    let mut supply = supply;

    if supply == 0 {
        return;
    }

    let mut bids_guard = state.bids.lock();

    if !bids_guard.is_empty() {
        let mut allocations_guard = state.allocations.lock();
        for (_price, queue) in bids_guard.iter_mut().rev() {
            let mut front_volume: Option<u64> = None;
            while supply > 0 && !queue.is_empty() {
                if let Some(mut front) = queue.peek_mut() {
                    // If supply > volume allocate volume.
                    // If volume > supply allocate supply.
                    let to_allocate = supply.min(front.volume);
                    *allocations_guard.entry(front.username.clone()).or_insert(0) += to_allocate;
                    front.volume -= to_allocate;
                    supply -= to_allocate;
                    front_volume = Some(front.volume);
                }
                if front_volume == Some(0) {
                    queue.pop();
                }
            }
            if supply == 0 {
                break;
            }
        }

        bids_guard.retain(|_, q| !q.is_empty());
    }

    drop(bids_guard);

    if supply > 0 {
        let mut supply_guard = state.supply.lock();
        *supply_guard += supply;
    }
}

#[post("/sell")]
pub async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> impl Responder {
    let supply = req.volume;
    handle_sell(&state, supply).await;

    HttpResponse::Ok()
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
