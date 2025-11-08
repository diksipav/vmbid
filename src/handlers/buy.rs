use crate::errors::TwinError;
use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, post, web};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

pub async fn handle_buy(
    state: &AppState,
    username: String,
    price: u64,
    volume: u64,
) -> Result<(), TwinError> {
    if username.is_empty() {
        return Err(TwinError::MissingUsername);
    }

    let mut volume = volume;
    if volume == 0 {
        return Ok(());
    }

    let mut to_allocate = 0;

    {
        let mut supply_guard = state.state.supply.lock().unwrap();
        if *supply_guard > 0 {
            let available = *supply_guard;
            to_allocate = volume.min(available);
            *supply_guard -= to_allocate;
            volume -= to_allocate;
        }
    }

    if to_allocate > 0 {
        let mut allocations_guard = state.state.allocations.lock().unwrap();
        *allocations_guard.entry(username.clone()).or_insert(0) += to_allocate;
    }

    if volume > 0 {
        let seq = state.state.seq.fetch_add(1, Ordering::Relaxed);
        let bid = Bid {
            username,
            price,
            volume,
            seq,
        };

        let mut bids_guard = state.state.bids.lock().unwrap();
        bids_guard
            .entry(price)
            .or_insert_with(VecDeque::new)
            .push_back(bid);
    }

    Ok(())
}

#[post("/buy")]
pub async fn buy(state: web::Data<AppState>, req: web::Json<BuyRequest>) -> impl Responder {
    let BuyRequest {
        username,
        price,
        volume,
    } = req.into_inner();

    match handle_buy(&state, username, price, volume).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::BadRequest().body("username can not be empty"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web;
    use std::collections::BTreeMap;

    // Creates a BTreeMap and populates it with one entry using provided data.
    fn create_bids(username: String, price: u64, volume: u64) -> BTreeMap<u64, VecDeque<Bid>> {
        let mut bids = BTreeMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(Bid {
            username,
            price,
            volume,
            seq: 0,
        });
        bids.insert(price, queue);
        return bids;
    }

    #[actix_web::test]
    async fn test_buy_with_no_supply_creates_bid() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 3, 100).await;
        assert!(result.is_ok());

        let expected = create_bids("u1".to_string(), 3, 100);
        let bids = state.state.bids.lock().unwrap();
        assert_eq!(*bids, expected);
    }

    #[actix_web::test]
    async fn test_buy_with_supply_immediate_allocation() {
        let state = AppState::default();
        // Set initial supply
        *state.state.supply.lock().unwrap() = 150;

        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 3, 100).await;
        assert!(result.is_ok());

        // Supply is updated. No bids.
        assert_eq!(state.state.bids.lock().unwrap().len(), 0);
        assert_eq!(*state.state.supply.lock().unwrap(), 50);

        // Allocation is created
        let allocations = state.state.allocations.lock().unwrap();
        assert_eq!(allocations.len(), 1);
        let allocation = allocations.get("u1").unwrap();
        assert_eq!(*allocation, 100);
    }

    #[actix_web::test]
    async fn test_buy_with_partial_supply() {
        let state = AppState::default();
        // Set initial supply
        *state.state.supply.lock().unwrap() = 50;

        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 4, 200).await;
        assert!(result.is_ok());

        // Required resources are higner than supply,
        // so both allocation and bid are created.
        // Supply is emptied.
        let expected = create_bids("u1".to_string(), 4, 150);
        assert_eq!(*state.state.bids.lock().unwrap(), expected);
        assert_eq!(*state.state.supply.lock().unwrap(), 0);

        let allocations = state.state.allocations.lock().unwrap();
        assert_eq!(allocations.len(), 1);
        let allocation = allocations.get("u1").unwrap();
        assert_eq!(*allocation, 50);
    }

    #[actix_web::test]
    async fn test_buy_with_0_volume() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        let result = handle_buy(&data, "u1".to_string(), 3, 0).await;
        assert!(result.is_ok());

        // Could also test if all locks have default values.
    }

    #[actix_web::test]
    #[should_panic(expected = "username cannot be empty")]
    async fn test_buy_with_empty_username() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        handle_buy(&data, "".to_string(), 3, 100).await.unwrap();
    }

    #[actix_web::test]
    async fn test_concurrent_buys_fifo_ordering() {
        let state = AppState::default();
        let data = web::Data::new(state.clone());

        // Spawn two concurrent buys
        let handle1 = {
            let data = data.clone();
            tokio::spawn(async move { handle_buy(&data, "u1".to_string(), 5, 100).await })
        };

        let handle2 = {
            let data = data.clone();
            tokio::spawn(async move { handle_buy(&data, "u2".to_string(), 5, 50).await })
        };

        // Wait for both to complete
        handle1.await.unwrap().unwrap();
        handle2.await.unwrap().unwrap();
        // assert!(handle1.await.unwrap().is_ok());
        // assert!(handle2.await.unwrap().is_ok());

        // Assert that there is a queue created for
        // the price 5 with sequence increasing.
        let bids = state.state.bids.lock().unwrap();
        let queue = bids.get(&5).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.get(0).unwrap().seq, 0);
        assert_eq!(queue.get(1).unwrap().seq, 1);
    }
}
