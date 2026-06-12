use application::errors::ApplicationError;
use application::users::create_user::PasswordHasher;
use argon2::{
    Argon2, PasswordHash as ArgonPasswordHash, PasswordHasher as ArgonPasswordHasher,
    PasswordVerifier,
};
use async_trait::async_trait;
use password_hash::SaltString;
use rand::rngs::OsRng;

/// OWASP-recommended Argon2id hasher.
pub struct Argon2PasswordHasher;

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash_password(&self, password: &str) -> Result<String, ApplicationError> {
        let password = password.to_string();
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))
        })
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, ApplicationError> {
        let password = password.to_string();
        let hash = hash.to_string();
        tokio::task::spawn_blocking(move || {
            let parsed = ArgonPasswordHash::new(&hash)
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
            match Argon2::default().verify_password(password.as_bytes(), &parsed) {
                Ok(()) => Ok(true),
                Err(password_hash::Error::Password) => Ok(false),
                Err(e) => Err(ApplicationError::Infrastructure(e.to_string())),
            }
        })
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
    }
}
