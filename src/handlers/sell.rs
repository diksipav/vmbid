use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, post, web};

pub async fn handle_sell(state: &AppState, supply: u64) {
    let mut supply = supply;

    if supply == 0 {
        return;
    }

    let mut bids_guard = state.state.bids.lock().unwrap();

    if !bids_guard.is_empty() {
        let mut allocations_guard = state.state.allocations.lock().unwrap();
        for (_price, queue) in bids_guard.iter_mut().rev() {
            while supply > 0 && !queue.is_empty() {
                let front = queue.front_mut().unwrap();

                let to_allocate = supply.min(front.volume);
                *allocations_guard.entry(front.username.clone()).or_insert(0) += to_allocate;

                front.volume -= to_allocate;
                supply -= to_allocate;

                if front.volume == 0 {
                    queue.pop_front();
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
        let mut supply_guard = state.state.supply.lock().unwrap();
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
    use std::collections::{BTreeMap, VecDeque};

    #[actix_web::test]
    async fn test_sell_when_no_bids_creates_supply() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        handle_sell(&data, 100).await;

        assert_eq!(*state.state.supply.lock().unwrap(), 100);
    }

    #[actix_web::test]
    async fn test_sell_with_excess_volume() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        // Populate state bids with one element
        let mut bids = BTreeMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(Bid {
            username: "u1".to_string(),
            price: 5,
            volume: 100,
            seq: 0,
        });
        bids.insert(5, queue);
        *state.state.bids.lock().unwrap() = bids;

        handle_sell(&data, 200).await;

        assert_eq!(*state.state.supply.lock().unwrap(), 100);
        assert!(state.state.bids.lock().unwrap().is_empty());
        assert_eq!(
            *state.state.allocations.lock().unwrap().get("u1").unwrap(),
            100
        );
    }
}
