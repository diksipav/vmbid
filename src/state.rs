use std::{
    collections::{BTreeMap, BinaryHeap, HashMap, binary_heap::PeekMut},
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use log::info;
use parking_lot::Mutex;

use crate::models::Bid;

#[derive(Default)]
pub struct Inner {
    pub bids: Mutex<BTreeMap<u64, BinaryHeap<Bid>>>,
    pub supply: Mutex<u64>,
    pub allocations: Mutex<HashMap<String, u64>>,
    pub seq: AtomicU64,
}

/// Shared application state accessible in handlers
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
    /// Match volume user wants to buy with available supply and allocate to user.
    pub fn consume_supply_and_allocate(&self, username: &str, volume: u64) -> u64 {
        let mut supply_guard = self.supply.lock();
        let mut allocations_guard = self.allocations.lock();

        let matched = volume.min(*supply_guard);
        *supply_guard -= matched;

        if matched > 0 {
            *allocations_guard.entry(username.to_string()).or_insert(0) += matched;
            let entry = allocations_guard.entry(username.to_string()).or_insert(0);
            let prior = *entry;
            *entry += volume;

            info!("User {} allocation: {} -> {}", username, prior, prior + volume);
        }

        matched
    }

    /// Add volume to the supply.
    pub fn add_supply(&self, amount: u64) {
        let mut supply_guard = self.supply.lock();
        let prior = *supply_guard;
        *supply_guard += amount;

        info!("Supply increased: {} -> {}", prior, prior + amount);
    }

    /// Create a new bid and queue it for processing.
    pub fn queue_bid(&self, username: &str, volume: u64, price: u64) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let bid = Bid { username: username.to_string(), volume, price, seq };
        self.bids.lock().entry(price).or_insert_with(BinaryHeap::new).push(bid);

        info!("queued bid: user={} volume={} price={} seq={}", username, volume, price, seq);
    }

    /// Match volume to be sold with available bids and create appropriate allocations.
    /// Return remaining/unallocated volume.
    pub fn match_sell_with_bids(&self, volume: u64) -> u64 {
        let mut remaining = volume;
        let mut bids_guard = self.bids.lock();
        let mut allocations_guard = self.allocations.lock();

        // Match highest-price bids first (BTreeMap::rev()).
        // Empty the highest price queue before proceeding to following queues.
        for (_, queue) in bids_guard.iter_mut().rev() {
            while remaining > 0 {
                let Some(mut front) = queue.peek_mut() else {
                    break;
                };

                let matched = remaining.min(front.volume);
                let entry = allocations_guard.entry(front.username.clone()).or_insert(0);
                let prior = *entry;
                *entry += matched;
                info!("User {} allocation: {} -> {}", front.username, prior, prior + matched);

                remaining -= matched;

                // remove bid from the queue if fully matched/allocated
                if front.volume == matched {
                    PeekMut::pop(front);
                } else {
                    front.volume -= matched;
                }
            }

            // all provided volume is matched/allocated, we are done
            if remaining == 0 {
                break;
            }
        }

        // remove empty queues
        bids_guard.retain(|_, q| !q.is_empty());

        remaining
    }
}
