use crate::models::Bid;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ops::Deref;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Inner {
    pub bids: Mutex<BTreeMap<u64, VecDeque<Bid>>>,
    pub supply: Mutex<u64>,
    pub allocations: Mutex<HashMap<String, u64>>,
    pub seq: AtomicU64,
}

#[derive(Clone, Default)]
pub struct AppState {
    pub state: Arc<Inner>,
}

impl Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl AppState {
    // Calculates total sold and total bought volume in the system
    // TODO: This is used in the tests foolder only, what is the
    // most idiomatic approach, to do sth like this? Or to create
    // a separate common file inside tests folder?
    pub fn total_volume_in_the_system(&self) -> (u64, u64) {
        let allocations: u64 = self.allocations.lock().unwrap().values().sum();

        let supply = *self.supply.lock().unwrap();

        let open_bids: u64 = self
            .state
            .bids
            .lock()
            .unwrap()
            .values()
            .flat_map(|queue| queue.iter())
            .map(|bid| bid.volume)
            .sum();

        (allocations + supply, allocations + open_bids)
    }
}
