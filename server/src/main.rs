use actix_web::{App, HttpServer};

use crate::api::server::start_server;

mod db;
mod api;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=trace");
    env_logger::init();
    start_server()
        .await
}
