use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::fs;

async fn get_file() -> impl Responder {
    match fs::read_to_string("../files/example.txt") {
        Ok(content) => HttpResponse::Ok().content_type("text/plain").body(content),
        Err(_) => HttpResponse::InternalServerError().body("Failed to read file"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server running on http://127.0.0.1:8080");

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173") // dev frontend
            .allowed_methods(vec!["GET"])
            .allowed_headers(vec!["Content-Type"])
            .max_age(3600); // cache preflight response for 1 hour

        App::new()
            .wrap(cors)
            .route("/file", web::get().to(get_file))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}