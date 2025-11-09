#[derive(Debug)]
pub enum TwinError {
    MissingUsername,
    NotFound(String),
}
