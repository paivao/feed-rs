use std::fmt;
use std::fmt::Display;

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use argon2::password_hash;
use serde::Serialize;
use sqlx;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    InternalError,
    BadRequest,
    DB(sqlx::Error),
    Hashing(password_hash::errors::Error),
    InvalidPassword,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound => write!(f, "Not Found"),
            ApiError::InternalError => write!(f, "Internal Error"),
            ApiError::BadRequest => write!(f, "Bad Request"),
            ApiError::DB(e) => write!(f, "Database Error: {}", e),
            ApiError::Hashing(e) => write!(f, "Hashing Error: {}", e),
            ApiError::InvalidPassword => write!(f, "Invalid Password"),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(v: sqlx::Error) -> Self {
        Self::DB(v)
    }
}

impl From<password_hash::errors::Error> for ApiError {
    fn from(v: password_hash::errors::Error) -> Self {
        Self::Hashing(v)
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
}

/// A plain `(status, message)` API error. Renders as JSON — `{"error": "<message>"}` —
/// instead of the bare `text/plain` body actix's own `ErrorBadRequest`/`ErrorNotFound`/etc.
/// helpers produce, so the frontend has a consistent, parseable error shape.
#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    message: String,
}

impl HttpError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(ErrorBody {
            error: &self.message,
        })
    }
}

/// Shorthand for `HttpError::new(status, message).into()`.
pub fn json_error(status: StatusCode, message: impl Into<String>) -> actix_web::Error {
    HttpError::new(status, message).into()
}
