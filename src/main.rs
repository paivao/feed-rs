use actix_web::web;
use dotenvy::{dotenv, from_path};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::env;
use std::path::PathBuf;

use crate::log::logging_bootstrap;

mod auth;
mod controller;
mod error;
mod log;
mod model;
mod server;
mod utils;

const APP_NAME: &str = "feed-rs";

const DEFAULT_MAX_DB_CONNS: u32 = 5;
//const DEFAULT_DB: &str = "feedme";
const DEFAULT_BIND_PORT: u16 = 8080;

enum Command {
    Serve,
    Migrate,
    Upgrade,
}

struct Args {
    command: Command,
    env_file: Option<PathBuf>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = parse_args();

    match &args.env_file {
        Some(path) => {
            from_path(path).unwrap_or_else(|_| panic!("Configuration file {path:?} not found."));
        }
        None => {
            dotenv().expect(r#"Configuration ".env" file not found."#);
        }
    }
    logging_bootstrap(APP_NAME);

    match args.command {
        Command::Serve => {
            let pool = connect_db().await;
            run_migrations(pool.get_ref()).await;
            server::run(pool).await
        }
        Command::Migrate => {
            let pool = connect_db().await;
            run_migrations(pool.get_ref()).await;
            Ok(())
        }
        Command::Upgrade => todo!("version upgrade not implemented yet"),
    }
}

fn parse_args() -> Args {
    let mut env_file = None;
    let mut positional = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--env-file" | "-e" => {
                let path = args.next().unwrap_or_else(|| {
                    eprintln!("{arg} requires a path argument");
                    std::process::exit(1);
                });
                env_file = Some(PathBuf::from(path));
            }
            other => positional.push(other.to_string()),
        }
    }

    let command = match positional.first().map(String::as_str) {
        None | Some("serve") => Command::Serve,
        Some("migrate") => Command::Migrate,
        Some("upgrade") => Command::Upgrade,
        Some(other) => {
            eprintln!(
                "Unknown command: {other}\nUsage: feed-rs [--env-file <path>] [serve|migrate|upgrade]"
            );
            std::process::exit(1);
        }
    };

    Args { command, env_file }
}

async fn connect_db() -> web::Data<PgPool> {
    let conn_options = PgConnectOptions::new().application_name(APP_NAME);
    web::Data::new(
        PgPoolOptions::new()
            .max_connections(get_env_with_default(
                "DB_POOL_MAX_CONNS",
                DEFAULT_MAX_DB_CONNS,
            ))
            .connect_with(conn_options)
            .await
            .expect("Unable to connect to database"),
    )
}

async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations").run(pool).await.unwrap();
}

#[inline]
fn get_env_with_default<T>(var: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    env::var(var).map_or(default, |x| x.parse().unwrap_or(default))
}
