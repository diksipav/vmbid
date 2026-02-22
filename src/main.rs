use axum::{
    Router,
    routing::{get, post},
};
use env_logger::Env;
use log::info;

pub mod errors;
pub mod handlers;
pub mod models;
pub mod state;

use handlers::{allocation::allocation, buy::buy, sell::sell};
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // automatically log everything with info level or higher
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting server...");

    let state = AppState::default();

    let app = Router::new()
        .route("/allocation", get(allocation))
        .route("/buy", post(buy))
        .route("/sell", post(sell))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Listening on 0.0.0.0:8080");

    axum::serve(listener, app).await?;
    Ok(())
}
