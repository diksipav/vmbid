use actix_web::{App, HttpServer, web};
use env_logger::Env;
use log::info;

pub mod errors;
pub mod handlers;
pub mod models;
pub mod state;

use handlers::{allocation::allocation, buy::buy, sell::sell};
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // automatically log everything with info level or higher
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting server...");
    let state = AppState::default();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(buy)
            .service(sell)
            .service(allocation)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
