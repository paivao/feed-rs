use argon2::password_hash;
use sqlx;

pub enum ApiError {
    DB(sqlx::Error),
    Hashing(password_hash::errors::Error),
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
