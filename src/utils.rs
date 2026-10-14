use actix_web::http::StatusCode;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::error::ErrorKind;

use crate::error::{ApiError, json_error};

pub fn create_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let hash_object = match PasswordHash::new(hash) {
        Ok(hash_object) => hash_object,
        Err(e) => {
            crate::app_error!("Failed to initialize password hasher: {}", e);
            return false;
        }
    };
    match Argon2::default().verify_password(password.as_bytes(), &hash_object) {
        Ok(()) => true,
        Err(e) => {
            crate::app_debug!("Failed to verify password: {}", e);
            false
        }
    }
}

/// A random, URL-safe token suitable for use as a session id.
pub fn generate_session_token() -> String {
    SaltString::generate(&mut OsRng).to_string()
}

/// Maps `RowNotFound` to a 404 naming `entity` (e.g. "feed not found"), and any other
/// error to a generic 500 via `db_error`. `id` is only for the log line, never the
/// client-facing message, so we don't leak row ids or query details to the frontend.
pub fn not_found_or_db_error(entity: &'static str, id: i64, err: sqlx::Error) -> actix_web::Error {
    if let sqlx::Error::RowNotFound = err {
        crate::app_trace!("{} {} not found", entity, id);
        json_error(StatusCode::NOT_FOUND, format!("{entity} not found"))
    } else {
        db_error(entity, id, err)
    }
}

/// A constraint violation (unique/foreign key/not-null/check) is caused by the data the
/// client sent, not a server-side failure, so it gets a short, generic 400 reason
/// instead of being lumped in with real internal errors.
fn bad_input_reason(err: &sqlx::Error) -> Option<&'static str> {
    let sqlx::Error::Database(db_err) = err else {
        return None;
    };
    match db_err.kind() {
        ErrorKind::UniqueViolation => Some("already exists"),
        ErrorKind::ForeignKeyViolation => Some("references a value that doesn't exist"),
        ErrorKind::NotNullViolation => Some("is missing a required field"),
        ErrorKind::CheckViolation => Some("has an invalid value"),
        _ => None,
    }
}

/// Everything that isn't a bad-input constraint violation is a genuine internal/database
/// failure: log the full detail, but only ever tell the client "internal server error" —
/// never leak query or driver details to the frontend.
pub fn db_error(entity: &'static str, id: i64, err: sqlx::Error) -> actix_web::Error {
    if let Some(reason) = bad_input_reason(&err) {
        crate::app_warn!("Invalid input on {} {}: {:?}", entity, id, err);
        return json_error(StatusCode::BAD_REQUEST, format!("{entity} {reason}"));
    }
    crate::app_warn!("Database error on {} {}: {:?}", entity, id, err);
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}
