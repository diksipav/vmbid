use crate::models::Bid;
use std::collections::{BTreeMap, HashMap, VecDeque};
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
