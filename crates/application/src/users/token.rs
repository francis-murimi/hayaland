use crate::errors::ApplicationError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication context carried inside a validated token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
}

impl AuthContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Outbound port for generating authentication tokens.
#[async_trait]
pub trait TokenGenerator: Send + Sync {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError>;
}

/// Outbound port for validating authentication tokens.
#[async_trait]
pub trait TokenVerifier: Send + Sync {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError>;
}
