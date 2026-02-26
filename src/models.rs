use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct BuyRequest {
    #[validate(length(min = 2, max = 20, message = "username must be 2-20 characters"))]
    pub username: String,
    #[validate(range(min = 1, message = "volume must be greater than 0"))]
    pub volume: u64,
    #[validate(range(min = 1, message = "price must be greater than 0"))]
    pub price: u64,
}

#[derive(Serialize)]
pub struct BuyResponse {
    pub allocated: u64,
    pub queued: u64,
}

#[derive(Deserialize, Validate)]
pub struct SellRequest {
    #[validate(range(min = 1, message = "volume must be greater than 0"))]
    pub volume: u64,
}

#[derive(Serialize)]
pub struct SellResponse {
    pub allocated: u64,
}

#[derive(Deserialize, Validate)]
pub struct AllocationQuery {
    #[validate(length(min = 2, max = 20, message = "username must be 2-20 characters"))]
    pub username: String,
}

#[derive(Serialize)]
pub struct AllocationResponse {
    pub allocated: u64,
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
        // Reverse so BinaryHeap pops lowest seq first (FIFO)
        other.seq.cmp(&self.seq)
    }
}

impl PartialOrd for Bid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
