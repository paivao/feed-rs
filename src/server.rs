use actix_files as fs;
use actix_web::middleware::{Logger, from_fn};
use actix_web::web::{self, Redirect};
use actix_web::{App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use rustls::ServerConfig;
use rustls::pki_types::CertificateDer;
use sqlx::PgPool;
use std::env;
use std::fs::File;
use std::io::BufReader;

use crate::auth::{self, SessionStore};
use crate::{APP_NAME, DEFAULT_BIND_PORT, controller, get_env_with_default};

pub async fn run(pool: web::Data<PgPool>) -> std::io::Result<()> {
    let sessions = web::Data::new(SessionStore::default());
    let bind_addr = get_bind_addr();
    let tls_config = load_tls_config();

    if tls_config.is_some() {
        println!("Started server at https://{}:{}/", bind_addr.0, bind_addr.1);
    } else {
        println!("Started server at http://{}:{}/", bind_addr.0, bind_addr.1);
    }

    let server = HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(sessions.clone())
            .wrap(
                Logger::new(r#"%a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
                    .log_target(format!("{APP_NAME}::access")),
            )
            // Frontend service
            .route(
                "/",
                web::get().to(async || Redirect::to("/admin/").permanent()),
            )
            .service(
                fs::Files::new("/admin", "./public")
                    .index_file("index.html")
                    .redirect_to_slash_directory(),
            )
            // Feed list service - HTTP Basic auth, for appliances polling feeds
            .service(
                web::scope("/feed")
                    .wrap(HttpAuthentication::basic(auth::basic_validator))
                    .service(controller::feed::serve_feed),
            )
            // Actix doesn't fall through between sibling scopes with the same prefix,
            // so login/logout and the session-protected routes share one /api scope.
            .service(
                web::scope("/api")
                    .configure(controller::user::configure_auth_api)
                    .service(
                        web::scope("")
                            .wrap(from_fn(auth::session_validator))
                            .configure(controller::feed::configure_feed_api)
                            .configure(controller::entry::configure_entry_api)
                            .configure(controller::user::configure_user_api),
                    ),
            )
    });

    match tls_config {
        Some(config) => server.bind_rustls_0_23(bind_addr, config)?.run().await,
        None => server.bind_auto_h2c(bind_addr)?.run().await,
    }
}

fn get_bind_addr() -> (String, u16) {
    let host = env::var("BIND_HOST").unwrap_or(String::from("127.0.0.1"));
    let port = get_env_with_default("BIND_PORT", DEFAULT_BIND_PORT);
    (host, port)
}

/// Builds a rustls `ServerConfig` from `TLS_CERT_FILE` / `TLS_KEY_FILE` when both are set.
/// Absent either, the server falls back to plain HTTP.
fn load_tls_config() -> Option<ServerConfig> {
    let cert_path = env::var("TLS_CERT_FILE").ok()?;
    let key_path = env::var("TLS_KEY_FILE").ok()?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Unable to install rustls crypto provider");

    let cert_chain: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut BufReader::new(
        File::open(&cert_path)
            .unwrap_or_else(|_| panic!("Unable to open TLS_CERT_FILE {cert_path:?}")),
    ))
    .collect::<Result<_, _>>()
    .unwrap_or_else(|_| panic!("Invalid TLS certificate in {cert_path:?}"));

    let key = rustls_pemfile::private_key(&mut BufReader::new(
        File::open(&key_path)
            .unwrap_or_else(|_| panic!("Unable to open TLS_KEY_FILE {key_path:?}")),
    ))
    .unwrap_or_else(|_| panic!("Invalid TLS private key in {key_path:?}"))
    .unwrap_or_else(|| panic!("No private key found in {key_path:?}"));

    Some(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .expect("Invalid TLS certificate/key pair"),
    )
}
