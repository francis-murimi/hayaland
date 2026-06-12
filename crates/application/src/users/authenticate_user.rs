use crate::errors::ApplicationError;
use crate::users::create_user::PasswordHasher;
use crate::users::dto::{AuthenticateUserCommand, AuthenticateUserResult};
use crate::users::token::{AuthContext, TokenGenerator};
use domain::entities::Email;
use domain::repositories::{RoleRepository, UserRepository};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct AuthenticateUser {
    repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
    hasher: Arc<dyn PasswordHasher>,
    token_generator: Arc<dyn TokenGenerator>,
}

impl AuthenticateUser {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        role_repo: Arc<dyn RoleRepository>,
        hasher: Arc<dyn PasswordHasher>,
        token_generator: Arc<dyn TokenGenerator>,
    ) -> Self {
        Self {
            repo,
            role_repo,
            hasher,
            token_generator,
        }
    }

    #[instrument(skip(self, cmd), fields(email = %cmd.email))]
    pub async fn execute(
        &self,
        cmd: AuthenticateUserCommand,
    ) -> Result<AuthenticateUserResult, ApplicationError> {
        let email = Email::new(&cmd.email).map_err(ApplicationError::from)?;

        let user = self
            .repo
            .find_by_email(&email)
            .await?
            .ok_or(ApplicationError::InvalidCredentials)?;

        let valid = self
            .hasher
            .verify_password(&cmd.password, user.password_hash.as_str())
            .await?;
        if !valid {
            warn!(user_id = %user.id, "authentication failed: invalid password");
            return Err(ApplicationError::InvalidCredentials);
        }

        if !user.is_active {
            warn!(user_id = %user.id, "authentication failed: account inactive");
            return Err(ApplicationError::AccountInactive);
        }

        let scopes = self.resolve_scopes(&user.roles).await?;
        let ctx = AuthContext {
            user_id: user.id,
            scopes,
            roles: user.roles.clone(),
        };
        let token = self.token_generator.generate(&ctx).await?;
        info!(user_id = %user.id, roles = ?ctx.roles, scopes = ?ctx.scopes, "user authenticated");
        Ok(AuthenticateUserResult {
            user_id: user.id,
            token,
        })
    }

    async fn resolve_scopes(&self, roles: &[String]) -> Result<Vec<String>, ApplicationError> {
        let mut scopes = HashSet::new();
        for role in roles {
            if let Some(def) = self.role_repo.find_by_name(role).await? {
                scopes.extend(def.scopes);
            }
        }
        let mut scopes: Vec<_> = scopes.into_iter().collect();
        scopes.sort();
        Ok(scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        test_repo_with, test_user, FakeHasher, FakeRoleRepo, FakeTokenGenerator,
    };
    use std::sync::Arc;

    fn service(repo: Arc<dyn UserRepository>) -> AuthenticateUser {
        AuthenticateUser::new(
            repo,
            Arc::new(FakeRoleRepo),
            Arc::new(FakeHasher),
            Arc::new(FakeTokenGenerator),
        )
    }

    #[tokio::test]
    async fn returns_token_for_valid_credentials() {
        let user = test_user("auth@example.com", "auth", "password123");
        let repo = test_repo_with(user);

        let result = service(repo)
            .execute(AuthenticateUserCommand {
                email: "auth@example.com".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().token.starts_with("token-"));
    }

    #[tokio::test]
    async fn rejects_invalid_password() {
        let user = test_user("auth@example.com", "auth", "password123");
        let repo = test_repo_with(user);

        let result = service(repo)
            .execute(AuthenticateUserCommand {
                email: "auth@example.com".to_string(),
                password: "wrongpassword".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn rejects_unknown_email() {
        let repo = test_repo_with(test_user("other@example.com", "other", "password123"));

        let result = service(repo)
            .execute(AuthenticateUserCommand {
                email: "unknown@example.com".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn rejects_inactive_user() {
        let mut user = test_user("inactive@example.com", "inactive", "password123");
        user.is_active = false;
        let repo = test_repo_with(user);

        let result = service(repo)
            .execute(AuthenticateUserCommand {
                email: "inactive@example.com".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::AccountInactive)));
    }
}
