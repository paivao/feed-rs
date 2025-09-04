use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::env;
use dotenvy::dotenv;

mod model;

const MAX_DB_CONNS: u32 = 5;
const DEFAULT_DB: &str = "feedme";

#[rocket::main]
async fn main() {
    dotenv().expect(r#"Configuration ".env" file not found."#);
    let pool = PgPoolOptions::new()
        .max_connections(env::var("DB_POOL_MAX_CONNS").map_or(MAX_DB_CONNS, |x| x.parse().unwrap_or(MAX_DB_CONNS)))
        .connect_with(PgConnectOptions::new()).await.expect("Unable to connect to database");
    sqlx::migrate!("./migrations").run(&pool).await;
    println!("Hello, world!");
}
