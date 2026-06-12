use application::errors::ApplicationError;
use application::users::token::{AuthContext, TokenGenerator, TokenVerifier};
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    roles: Vec<String>,
    scope: Vec<String>,
    exp: usize,
    iat: usize,
}

/// JWT token service using HS256.
pub struct JwtTokenService {
    secret: String,
    expiry_seconds: i64,
}

impl JwtTokenService {
    pub fn new(secret: String, expiry_seconds: i64) -> Self {
        Self {
            secret,
            expiry_seconds,
        }
    }

    fn now(&self) -> Result<usize, ApplicationError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))
            .map(|d| d.as_secs() as usize)
    }
}

#[async_trait]
impl TokenGenerator for JwtTokenService {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError> {
        let now = self.now()?;

        let claims = Claims {
            sub: ctx.user_id.to_string(),
            roles: ctx.roles.clone(),
            scope: ctx.scopes.clone(),
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

#[async_trait]
impl TokenVerifier for JwtTokenService {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError> {
        let validation = Validation::default();

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|_| ApplicationError::Unauthorized)?;

        let user_id = token_data
            .claims
            .sub
            .parse()
            .map_err(|_| ApplicationError::Unauthorized)?;

        Ok(AuthContext {
            user_id,
            roles: token_data.claims.roles,
            scopes: token_data.claims.scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn service() -> JwtTokenService {
        JwtTokenService::new("test-secret".to_string(), 3600)
    }

    fn ctx() -> AuthContext {
        AuthContext {
            user_id: Uuid::now_v7(),
            roles: vec!["user".to_string()],
            scopes: vec!["users:read".to_string()],
        }
    }

    #[tokio::test]
    async fn generates_and_verifies_token() {
        let svc = service();
        let ctx = ctx();
        let token = svc.generate(&ctx).await.unwrap();
        let verified = svc.verify(&token).await.unwrap();
        assert_eq!(verified.user_id, ctx.user_id);
        assert_eq!(verified.roles, ctx.roles);
        assert_eq!(verified.scopes, ctx.scopes);
    }

    #[tokio::test]
    async fn rejects_invalid_token() {
        let svc = service();
        let result = svc.verify("not-a-token").await;
        assert!(matches!(result, Err(ApplicationError::Unauthorized)));
    }
}
