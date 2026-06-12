use application::errors::ApplicationError;
use application::users::token::TokenGenerator;
use async_trait::async_trait;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

/// JWT token generator using HS256.
pub struct JwtTokenGenerator {
    secret: String,
    expiry_seconds: i64,
}

impl JwtTokenGenerator {
    pub fn new(secret: String, expiry_seconds: i64) -> Self {
        Self {
            secret,
            expiry_seconds,
        }
    }
}

#[async_trait]
impl TokenGenerator for JwtTokenGenerator {
    async fn generate(&self, user_id: Uuid) -> Result<String, ApplicationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?
            .as_secs() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            iat: now,
            exp: now + self.expiry_seconds as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))
    }
}
