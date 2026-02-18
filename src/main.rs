use actix_web::{App, HttpServer, web};
use log::info;

pub mod errors;
pub mod handlers;
pub mod models;
pub mod state;

use handlers::{allocation::allocation, buy::buy, sell::sell};
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
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
