use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    ops::Deref,
    sync::Arc,
};

use log::info;
use parking_lot::Mutex;

use crate::{
    errors::{Result, VmbidError},
    models::Bid,
};

/// Order book: open bids, leftover supply and allocations. Kept under a single lock
/// so every buy/sell sees and updates all of them atomically.
#[derive(Default)]
pub struct Book {
    /// Open bids by price, FIFO within a price level.
    pub bids: BTreeMap<u64, VecDeque<Bid>>,
    /// Leftover supply from sells.
    pub supply: u64,
    /// Total allocated volume per user.
    pub allocations: HashMap<String, u64>,
}

#[derive(Default)]
pub struct Inner {
    pub book: Mutex<Book>,
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
    /// Allocate from leftover supply, queue the remainder as a bid.
    /// Return allocated volume.
    pub fn buy(&self, username: &str, volume: u64, price: u64) -> u64 {
        let mut book = self.book.lock();

        let matched = book.supply.min(volume);
        if matched > 0 {
            book.supply -= matched;
            allocate(&mut book.allocations, username, matched);
        }

        let remaining = volume - matched;
        if remaining > 0 {
            let bid = Bid { username: username.to_string(), volume: remaining, price };
            book.bids.entry(price).or_default().push_back(bid);

            info!("queued bid: user={} volume={} price={}", username, remaining, price);
        }

        matched
    }

    /// Match volume with open bids (highest price first, FIFO within a price level)
    /// and store the leftover as supply. Return allocated volume.
    pub fn sell(&self, volume: u64) -> Result<u64> {
        let mut guard = self.book.lock();

        let Book { bids, supply, allocations } = &mut *guard;

        // Reject before changing anything: in the worst case the whole volume
        // ends up as leftover supply.
        if supply.checked_add(volume).is_none() {
            return Err(VmbidError::SupplyOverflow);
        }

        let mut remaining = volume;

        // Match highest-price bids first (BTreeMap::rev()).
        // Empty the highest price queue before proceeding to following queues.
        for (_, queue) in bids.iter_mut().rev() {
            while remaining > 0 {
                let Some(front) = queue.front_mut() else {
                    break;
                };

                let matched = remaining.min(front.volume);
                allocate(allocations, &front.username, matched);
                remaining -= matched;

                // remove bid from the queue if fully matched/allocated
                if front.volume == matched {
                    queue.pop_front();
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
        bids.retain(|_, q| !q.is_empty());

        if remaining > 0 {
            let prior = *supply;
            *supply += remaining;
            info!("Supply increased: {} -> {}", prior, *supply);
        }

        Ok(volume - remaining)
    }

    /// Return total allocated volume for a user, if they have any allocation.
    pub fn allocation(&self, username: &str) -> Option<u64> {
        self.book.lock().allocations.get(username).copied()
    }

    /// Return (allocations + supply, allocations + open bids) from one consistent snapshot.
    /// Used to check volume conservation: these must equal total sold and total bought.
    pub fn total_volume_in_the_system(&self) -> (u64, u64) {
        let book = self.book.lock();
        let allocations: u64 = book.allocations.values().sum();
        let open_bids: u64 = book.bids.values().flatten().map(|bid| bid.volume).sum();

        (allocations + book.supply, allocations + open_bids)
    }
}

/// Add matched volume to user's allocation.
fn allocate(allocations: &mut HashMap<String, u64>, username: &str, matched: u64) {
    let entry = allocations.entry(username.to_string()).or_insert(0);
    let prior = *entry;
    *entry += matched;

    info!("User {} allocation: {} -> {}", username, prior, *entry);
}
