use actix_web::{App, HttpServer, web};

mod handlers;
mod models;
mod state;

use handlers::*;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server...");
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
