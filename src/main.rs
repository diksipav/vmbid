mod errors;
mod handlers;
mod models;
mod state;

use axum::{
    Router,
    routing::{get, post},
};
use env_logger::Env;
use handlers::{allocation::handle_allocation, buy::handle_buy, sell::handle_sell};
use log::info;
use state::AppState;

#[tokio::main]
async fn main() {
    // automatically log everything with info level or higher
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting server...");

    let state = AppState::default();

    let app = Router::new()
        .route("/allocation", get(handle_allocation))
        .route("/buy", post(handle_buy))
        .route("/sell", post(handle_sell))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("Listening on 0.0.0.0:8080");

    axum::serve(listener, app).await.unwrap();
}
