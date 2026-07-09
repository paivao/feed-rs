use argon2::password_hash;
use derive_more::derive::{Display, Error};
use sqlx;

#[derive(Debug, Display, Error)]
pub enum ApiError {
    InternalError,
    BadRequest,
    DB(sqlx::Error),
    Hashing(password_hash::errors::Error),
    InvalidPassword,
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
