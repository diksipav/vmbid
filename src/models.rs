use serde::Deserialize;

#[derive(Deserialize)]
pub struct BuyRequest {
    pub username: String,
    pub volume: u64,
    pub price: u64,
}

#[derive(Deserialize)]
pub struct SellRequest {
    pub volume: u64,
}

#[derive(Debug)]
pub struct Bid {
    pub username: String,
    pub volume: u64,
    pub price: u64,
    pub seq: u64,
}

#[derive(Deserialize)]
pub struct AllocationQuery {
    pub username: Option<String>,
}
