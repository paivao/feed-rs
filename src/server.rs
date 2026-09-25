use actix_files as fs;
use actix_web::middleware::{Logger, from_fn};
use actix_web::web::{self, Redirect};
use actix_web::{App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use sqlx::PgPool;
use std::env;

use crate::auth::{self, SessionStore};
use crate::{APP_NAME, DEFAULT_BIND_PORT, controller, get_env_with_default};

pub async fn run(pool: web::Data<PgPool>) -> std::io::Result<()> {
    let sessions = web::Data::new(SessionStore::default());

    HttpServer::new(move || {
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
            // Login/logout - unauthenticated, issue/clear the session cookie
            .service(web::scope("/api").configure(controller::user::configure_auth_api))
            // Admin API - cookie session auth
            .service(
                web::scope("/api")
                    .wrap(from_fn(auth::session_validator))
                    .configure(controller::feed::configure_feed_api)
                    .configure(controller::entry::configure_entry_api)
                    .configure(controller::user::configure_user_api),
            )
    })
    .bind_auto_h2c(get_bind_addr())?
    .run()
    .await
}

fn get_bind_addr() -> (String, u16) {
    let host = env::var("BIND_HOST").unwrap_or(String::from("127.0.0.1"));
    let port = get_env_with_default("BIND_PORT", DEFAULT_BIND_PORT);
    (host, port)
}
