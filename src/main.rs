use actix_web::cookie::time::UtcDateTime;
use actix_web::error::ErrorUnauthorized;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::dev::ServiceRequest;
use std::collections::HashMap;
use std::sync::Mutex;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::env;
use dotenvy::dotenv;
use actix_web::{App, Error, HttpMessage, HttpServer};
use actix_web::web::{self, Redirect};
use actix_web::middleware::Logger;
use actix_files as fs;

use crate::log::logging_bootstrap;
use crate::model::user::User;

mod model;
mod controller;
mod log;

const APP_NAME: &str = "feed-rs";

const DEFAULT_MAX_DB_CONNS: u32 = 5;
//const DEFAULT_DB: &str = "feedme";
const DEFAULT_BIND_PORT: u16 = 8080;



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // First, get configuration
    dotenv().expect(r#"Configuration ".env" file not found."#);
    // Second, connect to database
    let conn_options = PgConnectOptions::new()
        .application_name(APP_NAME);
    let pool = web::Data::new(PgPoolOptions::new()
        .max_connections(get_env_with_default("DB_POOL_MAX_CONNS", DEFAULT_MAX_DB_CONNS))
        .connect_with(conn_options).await.expect("Unable to connect to database"));
    // And to migrate structure
    sqlx::migrate!("./migrations").run(pool.get_ref()).await.unwrap();

    // Setting logging sinks
    logging_bootstrap(APP_NAME);

    // Bearer authentication
    let bearer_auth_middleware = HttpAuthentication::with_fn(bearer_validator);

    // And then, web serve
    HttpServer::new(move ||{
        App::new()
            .app_data(pool.clone())
            .app_data(Mutex::new(HashMap::<String, (UtcDateTime, User)>::new()))
            .wrap(Logger::new(r#"%a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
                .log_target(format!("{APP_NAME}::access")))
            // Frontend service
            .route("/", web::get().to(async ||{Redirect::to("/admin").permanent()}))
            .service(fs::Files::new("/admin", "./public"))
            // Feed list service
            .service(controller::feed::serve_feed)
            // API
            .service(web::scope("/api")
                .configure(controller::feed::configure_feed_api)
            )
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

async fn bearer_validator(
    req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let Some(credentials) = credentials else {
        if req.path().eq("/api/login") {
            return Ok(req);
        }
        return Err((ErrorUnauthorized("unauthorized"), req));
    };

    Ok(req)
}