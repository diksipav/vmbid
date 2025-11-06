use crate::models::*;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, get, post, web};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

#[post("/buy")]
pub async fn buy(state: web::Data<AppState>, req: web::Json<BuyRequest>) -> impl Responder {
    let BuyRequest {
        username,
        price,
        mut volume,
    } = req.into_inner();

    if volume == 0 {
        return HttpResponse::Ok().finish();
    }

    if username.is_empty() {
        return HttpResponse::BadRequest().body("username cannot be empty");
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

        println!("didi {:?}", bids_guard);
    }

    HttpResponse::Ok().finish()
}

#[post("/sell")]
pub async fn sell(state: web::Data<AppState>, req: web::Json<SellRequest>) -> impl Responder {
    let mut supply = req.volume;
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

    HttpResponse::Ok()
}

#[get("/allocation")]
pub async fn allocation(
    state: web::Data<AppState>,
    query: web::Query<AllocationQuery>,
) -> impl Responder {
    let Some(username) = &query.username else {
        return HttpResponse::BadRequest().body("missing 'username' query parameter");
    };

    let allocations_guard = state.state.allocations.lock().unwrap();
    println!("didi {:?}", allocations_guard);
    match allocations_guard.get(username) {
        Some(allocation) => HttpResponse::Ok()
            .content_type("text/plain")
            .body(allocation.to_string()),
        None => HttpResponse::NotFound().body(format!("username '{}' not found", username)),
    }
}
