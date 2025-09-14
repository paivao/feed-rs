use simplelog::SharedLogger;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::env;
use std::fs::{File, OpenOptions};
use dotenvy::dotenv;
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{self, Redirect};
use actix_web::middleware::Logger;
use actix_files as fs;

use crate::log::logging_bootstrap;

mod model;
mod controller;
mod log;

const APP_NAME: &str = "feed-rs";

const DEFAULT_MAX_DB_CONNS: u32 = 5;
//const DEFAULT_DB: &str = "feedme";
const DEFAULT_BIND_PORT: u16 = 8080;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // First, get configuration
    dotenv().expect(r#"Configuration ".env" file not found."#);
    // Second, connect to database
    let conn_options = PgConnectOptions::new()
        .application_name(APP_NAME);
    let pool = PgPoolOptions::new()
        .max_connections(get_env_with_default("DB_POOL_MAX_CONNS", DEFAULT_MAX_DB_CONNS))
        .connect_with(conn_options).await.expect("Unable to connect to database");
    // And to migrate structure
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Setting logging sinks
    logging_bootstrap(APP_NAME);

    // And then, web serve
    HttpServer::new(move ||{
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(Logger::new(r#"%a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
                .log_target(format!("{APP_NAME}-http-access")))
            // Frontend service
            .route("/", web::get().to(async ||{Redirect::to("/admin").permanent()}))
            .service(fs::Files::new("/admin", "./public"))
            // Feed list service
            .service(controller::feed::serve_feed)
            // API
            .service(web::scope("/api")
                .configure(controller::feed::configure_feed_api)
            )
            .service(hello)
    }).bind_auto_h2c(get_bind_addr())?.run().await
}

fn get_bind_addr() -> (String, u16) {
    let host = env::var("BIND_HOST").unwrap_or(String::from("127.0.0.1"));
    let port = get_env_with_default("BIND_PORT", DEFAULT_BIND_PORT);
    (host, port)
}

#[inline]
fn get_env_with_default<T>(var: &str, default: T) -> T where T: std::str::FromStr + Copy {
    env::var(var).map_or(default, |x| x.parse().unwrap_or(default))
}

