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

#[derive(Debug, PartialEq)]
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
