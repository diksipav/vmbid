use std::cmp::Ordering;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct BuyRequest {
    pub username: String, // TODO: Can username be any UTF-8 string?
    pub volume: u64,
    pub price: u64,
}

#[derive(Deserialize)]
pub struct SellRequest {
    pub volume: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Bid {
    pub username: String,
    pub volume: u64,
    pub price: u64,
    pub seq: u64,
}

impl Ord for Bid {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap used in bids
        // (lower seq = higher priority)
        other.seq.cmp(&self.seq)
    }
}

impl PartialOrd for Bid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Deserialize)]
pub struct AllocationQuery {
    pub username: Option<String>,
}
