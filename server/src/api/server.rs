use actix_web::dev::Server;
use actix_web::{App, HttpServer, middleware};
use crate::api::router::*;


pub(crate) async fn start_server() -> std::io::Result<()> {
    let server_address = "127.0.0.1:3005";
    println!("Server is running {}", server_address);
    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(Router::init_routes)
    })
        .bind(server_address)?
        .run()
        .await?;
    Ok(server)
}
