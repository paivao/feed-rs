use actix_web::body::MessageBody;
use actix_web::cookie::Cookie;
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::middleware::Next;
use actix_web::{Error, web};
use actix_web_httpauth::extractors::basic::BasicAuth;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::model::user::User;
use crate::utils::generate_session_token;

pub const SESSION_COOKIE: &str = "feed_rs_session";
const SESSION_TTL_HOURS: i64 = 12;

/// Session token -> (expiry, logged in user). Shared across workers via `web::Data`.
pub type SessionStore = Mutex<HashMap<String, (OffsetDateTime, User)>>;

/// HTTP Basic authentication, checked against the users table on every request.
/// Meant for machine clients (e.g. appliances polling a feed) that don't want to deal
/// with cookies.
pub async fn basic_validator(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let Some(pool) = req.app_data::<web::Data<PgPool>>().cloned() else {
        return Err((ErrorInternalServerError("missing db pool"), req));
    };
    let password = credentials.password().unwrap_or_default();

    match User::verify_credentials(&pool, credentials.user_id(), password).await {
        Ok(_) => Ok(req),
        Err(_) => Err((ErrorUnauthorized("invalid credentials"), req)),
    }
}

/// Cookie-session authentication for the admin API: requires a valid, non-expired
/// session created by `POST /api/login`.
pub async fn session_validator(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let token = req.cookie(SESSION_COOKIE).map(|c| c.value().to_string());

    let is_valid = match (token, req.app_data::<web::Data<SessionStore>>()) {
        (Some(token), Some(sessions)) => {
            let sessions = sessions.lock().unwrap();
            sessions
                .get(&token)
                .is_some_and(|(expires_at, _)| *expires_at > OffsetDateTime::now_utc())
        }
        _ => false,
    };

    if !is_valid {
        return Err(ErrorUnauthorized("unauthorized"));
    }

    next.call(req).await
}

/// Create a new session for `user`, store it, and build the cookie to send back.
pub fn issue_session_cookie(sessions: &SessionStore, user: User) -> Cookie<'static> {
    let token = generate_session_token();
    let expires_at = OffsetDateTime::now_utc() + Duration::hours(SESSION_TTL_HOURS);
    sessions
        .lock()
        .unwrap()
        .insert(token.clone(), (expires_at, user));

    let mut cookie = Cookie::new(SESSION_COOKIE, token);
    cookie.set_path("/api");
    cookie.set_http_only(true);
    cookie.set_same_site(actix_web::cookie::SameSite::Strict);
    cookie.set_expires(expires_at);
    cookie
}

/// Drop `token`'s session (if any) and build a cookie that clears it client-side.
pub fn clear_session_cookie(sessions: &SessionStore, token: Option<&str>) -> Cookie<'static> {
    if let Some(token) = token {
        sessions.lock().unwrap().remove(token);
    }

    let mut cookie = Cookie::new(SESSION_COOKIE, "");
    cookie.set_path("/");
    cookie.set_max_age(Duration::ZERO);
    cookie
}
