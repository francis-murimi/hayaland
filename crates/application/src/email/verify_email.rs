use crate::email::dto::{VerifyEmailCommand, VerifyEmailResult};
use crate::errors::ApplicationError;
use domain::repositories::{EmailVerificationRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct VerifyEmail {
    user_repo: Arc<dyn UserRepository>,
    verification_repo: Arc<dyn EmailVerificationRepository>,
}

impl VerifyEmail {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        verification_repo: Arc<dyn EmailVerificationRepository>,
    ) -> Self {
        Self {
            user_repo,
            verification_repo,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(
        &self,
        cmd: VerifyEmailCommand,
    ) -> Result<VerifyEmailResult, ApplicationError> {
        let now = OffsetDateTime::now_utc();
        let verification = self
            .verification_repo
            .find_by_token(&cmd.token)
            .await?
            .filter(|v| v.is_valid(now))
            .ok_or(ApplicationError::InvalidOrExpiredVerificationToken)?;

        let mut user = self
            .user_repo
            .find_by_id(verification.user_id)
            .await?
            .ok_or(ApplicationError::InvalidOrExpiredVerificationToken)?;

        if user.is_active {
            warn!(user_id = %user.id, "verification attempted for already active user");
            return Err(ApplicationError::AlreadyVerified);
        }

        user.is_active = true;
        user.updated_at = now;
        self.user_repo.update(&user).await?;
        self.verification_repo.mark_used(&cmd.token).await?;

        info!(user_id = %user.id, "email verified and user activated");
        Ok(VerifyEmailResult { user_id: user.id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_user, FakeEmailVerificationRepo, FakeRepo};
    use std::sync::Arc;

    fn service(
        user_repo: Arc<dyn UserRepository>,
        verification_repo: Arc<dyn EmailVerificationRepository>,
    ) -> VerifyEmail {
        VerifyEmail::new(user_repo, verification_repo)
    }

    #[tokio::test]
    async fn activates_user_and_marks_token_used() {
        let mut user = test_user("verify@example.com", "verify", "password123");
        user.is_active = false;
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let token = "valid-token".to_string();
        verification_repo
            .save(&domain::entities::EmailVerification::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() + time::Duration::hours(24),
            ))
            .await
            .unwrap();

        let result = service(user_repo.clone(), verification_repo.clone())
            .execute(VerifyEmailCommand { token })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().user_id, user_id);
        assert!(
            user_repo
                .find_by_id(user_id)
                .await
                .unwrap()
                .unwrap()
                .is_active
        );
        assert!(
            verification_repo
                .find_by_token("valid-token")
                .await
                .unwrap()
                .unwrap()
                .used
        );
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let user = test_user("verify@example.com", "verify", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let token = "expired-token".to_string();
        verification_repo
            .save(&domain::entities::EmailVerification::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() - time::Duration::seconds(1),
            ))
            .await
            .unwrap();

        let result = service(user_repo, verification_repo)
            .execute(VerifyEmailCommand { token })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::InvalidOrExpiredVerificationToken)
        ));
    }

    #[tokio::test]
    async fn rejects_already_verified_user() {
        let mut user = test_user("verify@example.com", "verify", "password123");
        user.is_active = true;
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let token = "token".to_string();
        verification_repo
            .save(&domain::entities::EmailVerification::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() + time::Duration::hours(24),
            ))
            .await
            .unwrap();

        let result = service(user_repo, verification_repo)
            .execute(VerifyEmailCommand { token })
            .await;

        assert!(matches!(result, Err(ApplicationError::AlreadyVerified)));
    }
}
