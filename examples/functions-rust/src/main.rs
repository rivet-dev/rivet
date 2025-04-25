use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::env;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Find port
    let port_str = env::var("PORT_HTTP").expect("missing PORT_HTTP");
    let port = port_str.parse::<u16>().expect("invalid PORT_HTTP");

    println!("Listening on port {}", port);

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/health", web::get().to(health))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}