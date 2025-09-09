use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::env;
use dotenvy::dotenv;
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{self, Redirect};
use actix_files as fs;

mod model;
mod controller;

const MAX_DB_CONNS: u32 = 5;
const DEFAULT_DB: &str = "feedme";
const DEFAULT_PORT: u16 = 8080;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // First, get configuration
    dotenv().expect(r#"Configuration ".env" file not found."#);
    // Second, connect to database
    let pool = PgPoolOptions::new()
        .max_connections(env::var("DB_POOL_MAX_CONNS").map_or(MAX_DB_CONNS, |x| x.parse().unwrap_or(MAX_DB_CONNS)))
        .connect_with(PgConnectOptions::new()).await.expect("Unable to connect to database");
    // And to migrate structure
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // And then, web serve
    HttpServer::new(move ||{
        App::new()
            .app_data(web::Data::new(pool.clone()))
            // Frontend service
            .route("/", web::get().to(async ||{Redirect::to("/admin").permanent()}))
            .service(fs::Files::new("/admin", "./public"))
            // Feed service
            .service(controller::feed::serve_feed)
            .service(hello)
    }).bind_auto_h2c(get_bind_addr())?.run().await
}

fn get_bind_addr() -> (String, u16) {
    let host = env::var("BIND_ADDR").unwrap_or(String::from("127.0.0.1"));
    let port = env::var("").map_or(DEFAULT_PORT, |x| x.parse().unwrap_or(DEFAULT_PORT));
    (host, port)
}
