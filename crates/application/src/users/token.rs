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

/// Derive OAuth-style scopes from a list of roles.
pub fn scopes_from_roles(roles: &[String]) -> Vec<String> {
    let mut scopes = vec!["users:read".to_string()];
    if roles.iter().any(|r| r == "user") {
        scopes.push("users:write".to_string());
    }
    if roles.iter().any(|r| r == "admin") {
        scopes.extend_from_slice(&[
            "users:write".to_string(),
            "users:admin".to_string(),
            "users:delete".to_string(),
        ]);
    }
    scopes.sort();
    scopes.dedup();
    scopes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_user_role_gets_read_and_write_scopes() {
        let scopes = scopes_from_roles(&["user".to_string()]);
        assert!(scopes.contains(&"users:read".to_string()));
        assert!(scopes.contains(&"users:write".to_string()));
        assert!(!scopes.contains(&"users:admin".to_string()));
    }

    #[test]
    fn admin_role_gets_all_scopes() {
        let scopes = scopes_from_roles(&["admin".to_string()]);
        assert!(scopes.contains(&"users:read".to_string()));
        assert!(scopes.contains(&"users:write".to_string()));
        assert!(scopes.contains(&"users:admin".to_string()));
        assert!(scopes.contains(&"users:delete".to_string()));
    }
}
