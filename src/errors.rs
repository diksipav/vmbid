#[derive(Debug)]
pub enum VMbidError {
    MissingUsername,
    NotFound(String),
}
