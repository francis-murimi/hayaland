use crate::errors::ApplicationError;
use crate::users::dto::{CreateUserCommand, CreateUserResult};
use async_trait::async_trait;
use domain::entities::{Email, PasswordHash, User, Username};
use domain::repositories::UserRepository;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Outbound port for password hashing. The domain never sees plaintext passwords.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, ApplicationError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, ApplicationError>;
}

/// Create-user use case.
pub struct CreateUser {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl CreateUser {
    pub fn new(repo: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { repo, hasher }
    }

    #[instrument(skip(self, cmd), fields(email = %cmd.email, username = %cmd.username))]
    pub async fn execute(
        &self,
        cmd: CreateUserCommand,
    ) -> Result<CreateUserResult, ApplicationError> {
        let email = Email::new(&cmd.email).map_err(ApplicationError::from)?;
        let username = Username::new(&cmd.username).map_err(ApplicationError::from)?;
        validate_password(&cmd.password)?;

        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(ApplicationError::DuplicateEmail);
        }
        if self.repo.find_by_username(&username).await?.is_some() {
            return Err(ApplicationError::DuplicateUsername);
        }

        let hash = self.hasher.hash_password(&cmd.password).await?;
        let password_hash = PasswordHash::new(hash).map_err(ApplicationError::from)?;
        let id = Uuid::now_v7();
        let user = User::new(id, email, username, password_hash);

        self.repo.create(&user).await?;
        info!(%id, "created user");
        Ok(CreateUserResult { id })
    }
}

fn validate_password(password: &str) -> Result<(), ApplicationError> {
    if password.len() < 8 {
        return Err(ApplicationError::WeakPassword {
            message: "password must be at least 8 characters".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{FakeHasher, FakeRepo};
    use std::sync::Arc;

    fn service() -> CreateUser {
        CreateUser::new(
            Arc::new(FakeRepo {
                users: Default::default(),
            }),
            Arc::new(FakeHasher),
        )
    }

    #[tokio::test]
    async fn creates_user() {
        let result = service()
            .execute(CreateUserCommand {
                email: "alice@example.com".to_string(),
                username: "alice".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rejects_duplicate_email() {
        let svc = service();
        svc.execute(CreateUserCommand {
            email: "bob@example.com".to_string(),
            username: "bob".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

        let result = svc
            .execute(CreateUserCommand {
                email: "bob@example.com".to_string(),
                username: "bob2".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::DuplicateEmail)));
    }

    #[tokio::test]
    async fn rejects_duplicate_username() {
        let svc = service();
        svc.execute(CreateUserCommand {
            email: "carol@example.com".to_string(),
            username: "carol".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

        let result = svc
            .execute(CreateUserCommand {
                email: "carol2@example.com".to_string(),
                username: "carol".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::DuplicateUsername)));
    }

    #[tokio::test]
    async fn rejects_short_password() {
        let result = service()
            .execute(CreateUserCommand {
                email: "dave@example.com".to_string(),
                username: "dave".to_string(),
                password: "short".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::WeakPassword { .. })));
    }
}
