use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::error::ApiError;

pub fn create_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let Ok(hash_object) = PasswordHash::new(hash) else {
        return false;
    };
    match Argon2::default().verify_password(password.as_bytes(), &hash_object) {
        Ok(_) => true,
        Err(_) => {
            // TODO: logging
            false
        }
    }
}
