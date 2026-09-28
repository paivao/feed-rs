use std::fmt::Display;

use argon2::password_hash;
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
