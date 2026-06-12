use crate::email::{build_verification_email, generate_verification_token, EmailSender};
use crate::errors::ApplicationError;
use crate::users::dto::{CreateUserCommand, CreateUserResult};
use async_trait::async_trait;
use domain::entities::{Email, EmailVerification, PasswordHash, User, Username};
use domain::repositories::{EmailVerificationRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Outbound port for password hashing. The domain never sees plaintext passwords.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, ApplicationError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, ApplicationError>;
}

/// Create-user use case.
#[derive(Clone)]
pub struct CreateUser {
    repo: Arc<dyn UserRepository>,
    verification_repo: Arc<dyn EmailVerificationRepository>,
    email_sender: Arc<dyn EmailSender>,
    hasher: Arc<dyn PasswordHasher>,
    base_url: String,
    token_expiry_seconds: i64,
}

impl CreateUser {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        verification_repo: Arc<dyn EmailVerificationRepository>,
        email_sender: Arc<dyn EmailSender>,
        hasher: Arc<dyn PasswordHasher>,
        base_url: String,
        token_expiry_seconds: i64,
    ) -> Self {
        Self {
            repo,
            verification_repo,
            email_sender,
            hasher,
            base_url,
            token_expiry_seconds,
        }
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
        let mut user = User::new(id, email, username, password_hash);

        let is_first = self.repo.count().await? == 0;
        if is_first {
            user.roles = vec!["admin".to_string()];
            user.protected = true;
            info!(%id, "first user registered as protected admin");
        }

        // Every account, including the first admin, must verify its email.
        user.is_active = false;

        self.repo.create(&user).await?;

        let token = generate_verification_token();
        let expires_at =
            OffsetDateTime::now_utc() + time::Duration::seconds(self.token_expiry_seconds);
        self.verification_repo
            .save(&EmailVerification::new(token.clone(), id, expires_at))
            .await?;

        let expiry_hours = self.token_expiry_seconds / 3600;
        let (subject, body) = build_verification_email(&self.base_url, &token, expiry_hours);
        if let Err(e) = self
            .email_sender
            .send(user.email.as_str(), &subject, &body)
            .await
        {
            warn!(user_id = %id, error = %e, "failed to send verification email");
            return Err(ApplicationError::EmailSendFailed);
        }

        info!(%id, roles = ?user.roles, protected = user.protected, "created user pending email verification");
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
    use crate::test_helpers::{FakeEmailSender, FakeEmailVerificationRepo, FakeHasher, FakeRepo};
    use std::sync::Arc;

    fn service() -> CreateUser {
        CreateUser::new(
            Arc::new(FakeRepo {
                users: Default::default(),
            }),
            Arc::new(FakeEmailVerificationRepo::default()),
            Arc::new(FakeEmailSender::default()),
            Arc::new(FakeHasher),
            "https://app.hayaland.local".to_string(),
            86400,
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
    async fn first_user_becomes_protected_admin_but_inactive() {
        let svc = service();
        let result = svc
            .execute(CreateUserCommand {
                email: "first@example.com".to_string(),
                username: "first".to_string(),
                password: "password123".to_string(),
            })
            .await
            .unwrap();

        let repo = svc.repo.clone();
        let user = repo.find_by_id(result.id).await.unwrap().unwrap();
        assert!(user.has_role("admin"));
        assert!(user.protected);
        assert!(!user.is_active);
    }

    #[tokio::test]
    async fn subsequent_users_are_regular_users_and_inactive() {
        let svc = service();
        svc.execute(CreateUserCommand {
            email: "first@example.com".to_string(),
            username: "first".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

        let result = svc
            .execute(CreateUserCommand {
                email: "second@example.com".to_string(),
                username: "second".to_string(),
                password: "password123".to_string(),
            })
            .await
            .unwrap();

        let repo = svc.repo.clone();
        let user = repo.find_by_id(result.id).await.unwrap().unwrap();
        assert!(user.has_role("user"));
        assert!(!user.has_role("admin"));
        assert!(!user.protected);
        assert!(!user.is_active);
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

    #[tokio::test]
    async fn stores_verification_token() {
        let sender = Arc::new(FakeEmailSender::default());
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let svc = CreateUser::new(
            Arc::new(FakeRepo {
                users: Default::default(),
            }),
            verification_repo.clone(),
            sender.clone(),
            Arc::new(FakeHasher),
            "https://app.hayaland.local".to_string(),
            86400,
        );
        let result = svc
            .execute(CreateUserCommand {
                email: "token@example.com".to_string(),
                username: "token".to_string(),
                password: "password123".to_string(),
            })
            .await
            .unwrap();

        let body = sender.sent.lock().unwrap()[0].2.clone();
        let token = body
            .split("token=")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap()
            .to_string();
        let verification = verification_repo
            .find_by_token(&token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(verification.user_id, result.id);
        assert!(!verification.used);
    }

    #[tokio::test]
    async fn sends_verification_email() {
        let sender = Arc::new(FakeEmailSender::default());
        let svc = CreateUser::new(
            Arc::new(FakeRepo {
                users: Default::default(),
            }),
            Arc::new(FakeEmailVerificationRepo::default()),
            sender.clone(),
            Arc::new(FakeHasher),
            "https://app.hayaland.local".to_string(),
            86400,
        );

        svc.execute(CreateUserCommand {
            email: "email@example.com".to_string(),
            username: "email".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn returns_error_when_email_sending_fails() {
        let sender = Arc::new(FakeEmailSender::failing());
        let svc = CreateUser::new(
            Arc::new(FakeRepo {
                users: Default::default(),
            }),
            Arc::new(FakeEmailVerificationRepo::default()),
            sender.clone(),
            Arc::new(FakeHasher),
            "https://app.hayaland.local".to_string(),
            86400,
        );

        let result = svc
            .execute(CreateUserCommand {
                email: "fail@example.com".to_string(),
                username: "fail".to_string(),
                password: "password123".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::EmailSendFailed)));
    }
}
